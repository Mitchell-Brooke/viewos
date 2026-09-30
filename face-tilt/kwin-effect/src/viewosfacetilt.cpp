/*
 * SPDX-FileCopyrightText: 2026 ViewOS Project
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#include "viewosfacetilt.h"

#include <core/output.h>

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

Q_LOGGING_CATEGORY(VIEWOS_FACE_TILT, "kwin.viewosfacetilt", QtWarningMsg)

namespace KWin
{

ViewOSFaceTiltEffect::ViewOSFaceTiltEffect(QObject *parent)
    : Effect(parent)
    , m_client(new ViewOS::FaceClient(this))
{
    connect(m_client, &ViewOS::FaceClient::poseReceived,
            this, &ViewOSFaceTiltEffect::onPoseReceived);
    connect(m_client, &ViewOS::FaceClient::connectionChanged,
            this, &ViewOSFaceTiltEffect::onConnectionChanged);
}

ViewOSFaceTiltEffect::~ViewOSFaceTiltEffect() = default;

void ViewOSFaceTiltEffect::reconfigure(Effect::ReconfigureFlags flags)
{
    Effect::reconfigure(flags);
    loadConfig();
    if (m_enabled) {
        m_client->setSocketPath(QDir::temp().filePath("viewos-face-tilt.sock"));
        m_client->reconnectNow();
    } else {
        m_client->setSocketPath(QString());
        resetToNeutral();
    }
}

bool ViewOSFaceTiltEffect::isActive() const
{
    return m_enabled && m_havePose && m_client->isConnected();
}

void ViewOSFaceTiltEffect::prePaintScreen(ScreenPrePaintData &data, std::chrono::milliseconds presentTime)
{
    if (isActive()) {
        m_needsAnotherFrame = false;
        Effect::prePaintScreen(data, presentTime);
    }
}

void ViewOSFaceTiltEffect::paintScreen(const RenderTarget &renderTarget,
                                       const RenderViewport &viewport,
                                       int mask, const QRegion &region, Output *screen)
{
    Effect::paintScreen(renderTarget, viewport, mask, region, screen);
}

void ViewOSFaceTiltEffect::postPaintScreen()
{
    Effect::postPaintScreen();
    if (m_needsAnotherFrame) {
        EffectsHandler::self()->addRepaintFull();
    }
}

void ViewOSFaceTiltEffect::prePaintWindow(EffectWindow *w,
                                          WindowPrePaintData &data,
                                          std::chrono::milliseconds presentTime)
{
    if (!isEligible(w)) {
        Effect::prePaintWindow(w, data, presentTime);
        return;
    }

    data.setTransformed();

    if (!m_havePose || m_pose.confidence < m_minConfidence) {
        Effect::prePaintWindow(w, data, presentTime);
        return;
    }

    const QRectF geom = w->frameGeometry();
    const QPointF centreFraction(geom.center().x() / EffectsHandler::self()->screenSize().width(),
                                 geom.center().y() / EffectsHandler::self()->screenSize().height());

    QVector3D axis;
    float angle;
    targetRotation(centreFraction, &axis, &angle);

    const float smoothedAngle = angle * (1.0f - m_smoothing);
    if (!m_smoothed.contains(w->internalId())) {
        m_smoothed[w->internalId()] = {axis, smoothedAngle, true};
    } else {
        auto &s = m_smoothed[w->internalId()];
        s.axis = QVector3D::normal(s.axis + axis * (1.0f - m_smoothing));
        s.angle = s.angle * m_smoothing + smoothedAngle * (1.0f - m_smoothing);
        s.initialised = true;
    }

    if (!m_smoothed[w->internalId()].initialised) {
        Effect::prePaintWindow(w, data, presentTime);
        return;
    }

    const auto &s = m_smoothed[w->internalId()];
    data.setRotationAngle(qDegreesToRadians(s.angle));
    data.setRotationAxis(s.axis);
    data.setRotationOrigin(QVector3D(geom.center().x(), geom.center().y(), 0.0f));

    Effect::prePaintWindow(w, data, presentTime);
}

void ViewOSFaceTiltEffect::paintWindow(const RenderTarget &renderTarget,
                                       const RenderViewport &viewport,
                                       EffectWindow *w, int mask,
                                       QRegion region, WindowPaintData &data)
{
    Effect::paintWindow(renderTarget, viewport, w, mask, region, data);
}

void ViewOSFaceTiltEffect::loadConfig()
{
    KConfigGroup config = KSharedConfig::openConfig()->group("ViewOSFaceTilt");
    m_enabled = config.readEntry("Enabled", false);
    m_maxAngleDeg = config.readEntry("MaxAngle", 10.0f);
    m_minConfidence = config.readEntry("MinConfidence", 0.5f);
    m_smoothing = config.readEntry("Smoothing", 0.25f);
    m_gain = config.readEntry("Gain", 1.0f);
    m_invertYaw = config.readEntry("InvertYaw", false);
    m_invertPitch = config.readEntry("InvertPitch", false);

    if (m_enabled) {
        qCDebug(VIEWOS_FACE_TILT) << "ViewOS face tilt configured: enabled" << m_enabled
                                  << "maxAngle" << m_maxAngleDeg;
    } else {
        qCDebug(VIEWOS_FACE_TILT) << "ViewOS face tilt configured: disabled";
    }
}

bool ViewOSFaceTiltEffect::isEligible(EffectWindow *w) const
{
    if (!w || w->isDeleted() || w->isHidden() || w->isMinimized() || !w->isVisible() ||
        !w->isOnCurrentDesktop() || w->isDesktop() || w->isDock() || w->isToolbar() ||
        w->isMenu() || w->isDialog() || w->isSplash() || w->isUtility() ||
        w->isDropdownMenu() || w->isPopupMenu() || w->isTooltip() ||
        w->isNotification() || w->isCriticalNotification() || w->isAppletPopup() ||
        w->isOnScreenDisplay() || w->isComboBox() || w->isDNDIcon() ||
        w->isFullScreen() || w->isLockScreen() || w->isInputMethod() ||
        w->isPopupWindow() || w->isOutline() || w->isSpecialWindow()) {
        return false;
    }

    const QRectF frame = w->frameGeometry();
    const QRectF output = w->output()->geometry();
    if (frame.width() >= output.width() * 0.99 &&
        frame.height() >= output.height() * 0.99) {
        return false;
    }

    return true;
}

void ViewOSFaceTiltEffect::onPoseReceived(const ViewOS::HeadPose &pose)
{
    if (!m_havePose || std::abs(pose.hx - m_pose.hx) > 0.001 ||
        std::abs(pose.hy - m_pose.hy) > 0.001) {
        m_needsAnotherFrame = true;
    }
    m_havePose = true;
    m_pose = pose;
}

void ViewOSFaceTiltEffect::onConnectionChanged(bool connected)
{
    if (!connected && m_havePose) {
        m_havePose = false;
        m_needsAnotherFrame = true;
        qCDebug(VIEWOS_FACE_TILT) << "face-tilt daemon connection" << (connected ? "established" : "lost");
    }
}

void ViewOSFaceTiltEffect::resetToNeutral()
{
    m_havePose = false;
    m_needsAnotherFrame = true;
    m_smoothed.clear();
}

void ViewOSFaceTiltEffect::targetRotation(const QPointF &centreFraction,
                                          QVector3D *axis, float *angle) const
{
    if (!m_havePose) {
        *axis = QVector3D(0.0f, 0.0f, 1.0f);
        *angle = 0.0f;
        return;
    }

    const float dx = (m_pose.hx - centreFraction.x()) * m_gain;
    const float dy = (m_pose.hy - centreFraction.y()) * m_gain;

    *axis = QVector3D(-dy, -dx, 0.0f);
    float len = std::sqrt(dx * dx + dy * dy);
    *angle = std::min(len * 180.0f / M_PI, float(m_maxAngleDeg));

    if (m_invertYaw) axis->setX(-axis->x());
    if (m_invertPitch) axis->setY(-axis->y());
}

} // namespace KWin

// Static factory function for the plugin.
// Must match CreateInstanceWithMetaDataFunction exactly:
// QObject* (*)(QWidget*, QObject*, const KPluginMetaData&, const QList<QVariant>&)
static QObject* createViewOSFaceTiltEffect(QWidget *widget, QObject *parent,
                                           const KPluginMetaData &data, const QList<QVariant> &args)
{
    Q_UNUSED(widget);
    Q_UNUSED(data);
    Q_UNUSED(args);
    return new KWin::ViewOSFaceTiltEffect(parent);
}

// Manual plugin factory to avoid K_PLUGIN_FACTORY_WITH_JSON compile-time checks.
// Uses the public registerPlugin(CreateInstanceWithMetaDataFunction) overload.
class ViewOSFaceTiltEffectFactory : public KPluginFactory
{
    Q_OBJECT
public:
    ViewOSFaceTiltEffectFactory(QObject *parent = nullptr, const QVariantList &args = {})
        : KPluginFactory()
    {
        // Register using the public template overload with explicit template argument.
        // This avoids the template deduction failure with function pointers.
        registerPlugin<KWin::ViewOSFaceTiltEffect>(createViewOSFaceTiltEffect);
    }

    };

#include "viewosfacetilt.moc"
