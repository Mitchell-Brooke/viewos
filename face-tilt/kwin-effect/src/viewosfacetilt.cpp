/*
 * SPDX-FileCopyrightText: 2026 ViewOS Project
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#include "viewosfacetilt.h"

#include <kwin/core/output.h>

#include <KConfigGroup>
#include <KPluginFactory>
#include <KSharedConfig>

#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QMatrix4x4>
#include <QQuaternion>
#include <QStandardPaths>

#include <cmath>

namespace KWin
{

Q_LOGGING_CATEGORY(VIEWOS_FACE_TILT, "kwin.viewosfacetilt", QtWarningMsg)

namespace
{
constexpr qreal degreesToRadians(qreal deg)
{
    return deg * M_PI / 180.0;
}
}

ViewOSFaceTiltEffect::ViewOSFaceTiltEffect()
    : m_client(new ViewOS::FaceClient(this))
{
    reconfigure(ReconfigureAll);

    connect(m_client, &ViewOS::FaceClient::poseReceived, this, &ViewOSFaceTiltEffect::onPoseReceived);
    connect(m_client, &ViewOS::FaceClient::connectionChanged, this,
            &ViewOSFaceTiltEffect::onConnectionChanged);

    m_client->setSocketPath(
        QDir(QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation))
            .filePath(QStringLiteral("viewos/face-tilt.sock")));
}

ViewOSFaceTiltEffect::~ViewOSFaceTiltEffect() = default;

void ViewOSFaceTiltEffect::reconfigure(ReconfigureFlags flags)
{
    Effect::reconfigure(flags);
    loadConfig();
}

void ViewOSFaceTiltEffect::loadConfig()
{
    KConfigGroup cfg(KSharedConfig::openConfig(), QStringLiteral("Effect-ViewOSFaceTilt"));

    m_enabled = cfg.readEntry("Enabled", false);
    m_maxAngleDeg = cfg.readEntry("MaxAngle", 10.0f);
    m_minConfidence = cfg.readEntry("MinConfidence", 0.5f);
    m_smoothing = qBound(0.0f, cfg.readEntry("Smoothing", 0.25f), 0.95f);
    m_gain = cfg.readEntry("Gain", 1.0f);
    m_invertYaw = cfg.readEntry("InvertYaw", false);
    m_invertPitch = cfg.readEntry("InvertPitch", false);

    m_excludedWindows.clear();
    const QStringList excluded = cfg.readEntry("ExcludedWindows", QStringList());
    for (const QString &entry : excluded) {
        const QUuid id = QUuid::fromString(entry);
        if (!id.isNull()) {
            m_excludedWindows.insert(id);
        }
    }

    // Configuration may have switched the effect off; make sure any residual
    // transform is undone.
    if (!m_enabled) {
        resetToNeutral();
    }

    qCDebug(VIEWOS_FACE_TILT) << "ViewOS face tilt configured: enabled" << m_enabled << "maxAngle"
                          << m_maxAngleDeg << "smoothing" << m_smoothing;
}

bool ViewOSFaceTiltEffect::isActive() const
{
    return m_enabled && m_havePose;
}

void ViewOSFaceTiltEffect::onConnectionChanged(bool connected)
{
    qCDebug(VIEWOS_FACE_TILT) << "face-tilt daemon connection" << (connected ? "established" : "lost");
    if (!connected) {
        // Without a daemon there is no pose. Snap back to neutral and stop
        // repainting rather than freezing windows at their last angle.
        resetToNeutral();
    }
}

void ViewOSFaceTiltEffect::onPoseReceived(const ViewOS::HeadPose &pose)
{
    const bool hadPose = m_havePose;
    m_pose = pose;
    m_havePose = pose.isValid(m_minConfidence);

    if (!m_havePose) {
        // Tracking was lost. Ease back to neutral and stop once we get there.
        if (hadPose) {
            m_needsAnotherFrame = true;
            effects->addRepaintFull();
        }
        return;
    }

    m_needsAnotherFrame = true;
    effects->addRepaintFull();
}

void ViewOSFaceTiltEffect::resetToNeutral()
{
    m_havePose = false;
    if (m_smoothed.isEmpty()) {
        return;
    }
    m_smoothed.clear();
    m_needsAnotherFrame = true;
    effects->addRepaintFull();
}

void ViewOSFaceTiltEffect::targetRotation(const QPointF &centreFraction, QVector3D *axis, float *angle) const
{
    // Socket coordinates: origin top-left, +X right, +Y down. Convert to a
    // maths frame with +Y up so that a positive pitch tilts the top of the
    // window away from the viewer.
    const float dx = static_cast<float>(m_pose.hx - centreFraction.x());
    const float dy = static_cast<float>(centreFraction.y() - m_pose.hy);
    const float dz = static_cast<float>(m_pose.hz);

    // A head essentially level with, or behind, a window would demand a
    // rotation approaching 180 degrees, which is both visually useless and
    // would push the window's corners far off screen. Clamping dz keeps the
    // result well defined; the m_maxAngleDeg clamp then bounds it further.
    const float safeDz = qMax(dz, 50.0f);

    // Yaw about +Y, pitch about +X. The yaw sign is negative because rotating
    // about +Y by a positive angle turns the window's +Z normal towards -X.
    float yawDeg = -std::atan2(dx, safeDz) * 180.0f / static_cast<float>(M_PI);
    float pitchDeg = std::atan2(dy, safeDz) * 180.0f / static_cast<float>(M_PI);

    if (m_invertYaw) {
        yawDeg = -yawDeg;
    }
    if (m_invertPitch) {
        pitchDeg = -pitchDeg;
    }

    yawDeg *= m_gain;
    pitchDeg *= m_gain;

    const float limit = m_maxAngleDeg;
    yawDeg = qBound(-limit, yawDeg, limit);
    pitchDeg = qBound(-limit, pitchDeg, limit);

    // PaintData exposes rotation as a single axis-angle pair, so the yaw and
    // pitch rotations have to be composed into one. Order matters: yaw is
    // applied in the frame produced by pitch.
    const QQuaternion q = QQuaternion::fromAxisAngle(QVector3D(1.0f, 0.0f, 0.0f), pitchDeg)
        * QQuaternion::fromAxisAngle(QVector3D(0.0f, 1.0f, 0.0f), yawDeg);

    qreal axisAngle = 0.0;
    QVector3D axisOut(0.0f, 0.0f, 1.0f);
    q.toAxisAngle(&axisAngle, &axisOut);

    *axis = axisOut;
    // toAxisAngle returns degrees; PaintData wants radians.
    *angle = static_cast<float>(degreesToRadians(axisAngle));
}

bool ViewOSFaceTiltEffect::isEligible(EffectWindow *w) const
{
    if (!w || !m_enabled) {
        return false;
    }

    if (m_excludedWindows.contains(w->internalId())) {
        return false;
    }

    if (!w->isVisible() || w->isMinimized() || w->isHidden() || w->isDeleted()) {
        return false;
    }

    // Never move the lock screen or a screen saver.
    if (w->isLockScreen()) {
        return false;
    }
    const QString role = w->windowRole();
    if (role == QLatin1String("screen-saver") || role == QLatin1String("lock-screen")) {
        return false;
    }

    if (w->isFullScreen() || w->isDesktop() || w->isDock() || w->isSplash()
        || w->isOnScreenDisplay() || w->isInputMethod() || w->isDNDIcon() || w->isOutline()) {
        return false;
    }

    // Dialogs and transient UI: a tilted menu reads as detached from its
    // parent window and is harder to read.
    if (w->isDialog() || w->isPopupMenu() || w->isDropdownMenu() || w->isMenu() || w->isTooltip()
        || w->isUtility() || w->isToolbar() || w->isNotification() || w->isCriticalNotification()
        || w->isAppletPopup() || w->isComboBox() || w->isSpecialWindow()) {
        return false;
    }

    if (!w->isOnCurrentDesktop()) {
        return false;
    }

    // A maximised window presents as a frame that exactly fills its output.
    // Rotating it would either clip the content or move every menu and
    // scrollbar away from the pointer, so treat it as fullscreen.
    if (const auto outputs = w->outputs(); !outputs.empty()) {
        const QRect frame = w->frameGeometry().toRect();
        for (Output *output : outputs) {
            if (!output) {
                continue;
            }
            const QRect screenArea = output->geometry();
            if (!screenArea.isEmpty() && frame == screenArea) {
                return false;
            }
        }
    }

    return true;
}

void ViewOSFaceTiltEffect::prePaintScreen(ScreenPrePaintData &data, std::chrono::milliseconds presentTime)
{
    effects->prePaintScreen(data, presentTime);
}

void ViewOSFaceTiltEffect::paintScreen(const RenderTarget &renderTarget, const RenderViewport &viewport,
                                       int mask, const QRegion &region, Output *screen)
{
    effects->paintScreen(renderTarget, viewport, mask, region, screen);
}

void ViewOSFaceTiltEffect::prePaintWindow(EffectWindow *w, WindowPrePaintData &data,
                                          std::chrono::milliseconds presentTime)
{
    if (isActive() && isEligible(w)) {
        data.setTransformed();
    }
    effects->prePaintWindow(w, data, presentTime);
}

void ViewOSFaceTiltEffect::paintWindow(const RenderTarget &renderTarget, const RenderViewport &viewport,
                                       EffectWindow *w, int mask, QRegion region, WindowPaintData &data)
{
    if (!isActive() || !isEligible(w)) {
        effects->paintWindow(renderTarget, viewport, w, mask, region, data);
        return;
    }

    // Window centre as a fraction of the primary output, matching the
    // convention the daemon publishes.
    const auto outputs = w->outputs();
    Output *output = nullptr;
    for (Output *candidate : outputs) {
        if (candidate) {
            output = candidate;
            break;
        }
    }
    if (!output) {
        effects->paintWindow(renderTarget, viewport, w, mask, region, data);
        return;
    }

    const QRect screenArea = output->geometry();
    if (screenArea.isEmpty()) {
        effects->paintWindow(renderTarget, viewport, w, mask, region, data);
        return;
    }

    const QPoint centre = w->frameGeometry().toRect().center();
    const QPointF centreFraction(
        (static_cast<double>(centre.x()) - screenArea.x()) / screenArea.width(),
        (static_cast<double>(centre.y()) - screenArea.y()) / screenArea.height());

    QVector3D axis(0.0f, 0.0f, 1.0f);
    float angle = 0.0f;
    targetRotation(centreFraction, &axis, &angle);

    // Frame-rate-independent exponential smoothing. The factor is derived from
    // the elapsed time so that the response feels the same whether the daemon
    // publishes at 5 Hz or 30 Hz.
    Smoothed &state = m_smoothed[w->internalId()];
    if (!state.initialised) {
        state.axis = axis;
        state.angle = angle;
        state.initialised = true;
    } else {
        const float blend = m_smoothing;
        state.axis = state.axis * (1.0f - blend) + axis * blend;
        state.angle = state.angle * (1.0f - blend) + angle * blend;
    }

    // Rotate about the window's own centre, in the same coordinate space KWin
    // uses for window painting.
    const QRectF frame = w->frameGeometry();
    data.setRotationOrigin(QVector3D(static_cast<float>(frame.center().x()),
                                     static_cast<float>(frame.center().y()), 0.0f));
    data.setRotationAxis(state.axis);
    data.setRotationAngle(state.angle);

    effects->paintWindow(renderTarget, viewport, w, mask, region, data);
}

void ViewOSFaceTiltEffect::postPaintScreen()
{
    if (m_needsAnotherFrame) {
        m_needsAnotherFrame = false;
        effects->addRepaintFull();
    }
    effects->postPaintScreen();
}

} // namespace KWin

// Manually create the plugin factory with embedded metadata.json resource.
// This avoids K_PLUGIN_FACTORY_WITH_JSON macro issues with template checking.
class ViewOSFaceTiltEffectFactory : public KPluginFactory
{
    Q_OBJECT
public:
    ViewOSFaceTiltEffectFactory(QObject *parent = nullptr, const QVariantList &args = {})
        : KPluginFactory(parent, args)
    {
        // Use the function-pointer overload to avoid compile-time template checks.
        // The lambda creates an instance of our effect class.
        registerPlugin([](QObject *parent, const QVariantList &args) {
            return new KWin::ViewOSFaceTiltEffect(parent);
        });
    }

    QJsonObject metaData() const override
    {
        QFile file(QStringLiteral(":/metadata.json"));
        if (!file.open(QIODevice::ReadOnly)) {
            return {};
        }
        const QByteArray data = file.readAll();
        QJsonParseError error;
        const QJsonDocument doc = QJsonDocument::fromJson(data, &error);
        if (error.error != QJsonParseError::NoError || !doc.isObject()) {
            return {};
        }
        return doc.object();
    }
};

#include "viewosfacetilt.moc"
