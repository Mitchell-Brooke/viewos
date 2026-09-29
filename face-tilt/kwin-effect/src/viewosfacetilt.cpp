#include "viewosfacetilt.h"

#include <KConfigGroup>
#include <KSharedConfig>
#include <KWindowSystem>
#include <KWindowInfo>

#include <QFile>
#include <QLocalSocket>
#include <QStandardPaths>
#include <QDir>
#include <QDebug>

#include <unistd.h>
#include <sys/socket.h>
#include <sys/un.h>

namespace KWin
{

ViewOSFaceTiltEffect::ViewOSFaceTiltEffect()
    : Effect()
{
    // Initialize socket connection to face daemon
    const QString socketPath = QStringLiteral("/run/viewos/face-tilt.sock");

    m_socketFd = socket(AF_UNIX, SOCK_STREAM | SOCK_NONBLOCK, 0);
    if (m_socketFd >= 0) {
        struct sockaddr_un addr = {};
        addr.sun_family = AF_UNIX;
        strncpy(addr.sun_path, socketPath.toLocal8Bit().constData(), sizeof(addr.sun_path) - 1);

        if (connect(m_socketFd, (struct sockaddr*)&addr, sizeof(addr)) == 0) {
            m_socketNotifier = new QSocketNotifier(m_socketFd, QSocketNotifier::Read, this);
            connect(m_socketNotifier, &QSocketNotifier::activated, this, &ViewOSFaceTiltEffect::onSocketData);
            qCDebug(KWIN_EFFECTS) << "Connected to face daemon at" << socketPath;
        } else {
            close(m_socketFd);
            m_socketFd = -1;
            qCDebug(KWIN_EFFECTS) << "Face daemon not available at" << socketPath;
        }
    }

    // Config reload timer
    m_configTimer = new QTimer(this);
    m_configTimer->setInterval(5000);
    m_configTimer->setSingleShot(false);
    connect(m_configTimer, &QTimer::timeout, this, &ViewOSFaceTiltEffect::onConfigChanged);
    m_configTimer->start();

    // Initial config load
    reconfigure(ReconfigureAll);

    // Watch for window changes
    connect(effects, &EffectsHandler::windowAdded, this, [this](EffectWindow* w) {
        if (shouldTransformWindow(w)) {
            w->setData(WindowPaintData::Role, QVariant::fromValue(WindowPaintData()));
        }
    });
}

ViewOSFaceTiltEffect::~ViewOSFaceTiltEffect()
{
    if (m_socketFd >= 0) {
        close(m_socketFd);
    }
    delete m_socketNotifier;
    delete m_configTimer;
}

void ViewOSFaceTiltEffect::reconfigure(ReconfigureFlags flags)
{
    Effect::reconfigure(flags);

    KConfigGroup config = KSharedConfig::openConfig()->group("Effect-ViewOSFaceTilt");
    m_maxAngle = config.readEntry("MaxAngle", 12.0f);
    m_deadzone = config.readEntry("Deadzone", 2.0f);
    m_smoothing = config.readEntry("Smoothing", 0.15f);
    m_invertYaw = config.readEntry("InvertYaw", false);
    m_invertPitch = config.readEntry("InvertPitch", true);
    m_enabled = config.readEntry("Enabled", false);

    // Load exclusions
    QStringList excluded = config.readEntry("ExcludedWindows", QStringList());
    m_excludedWindowIds.clear();
    for (const QString& id : excluded) {
        m_excludedWindowIds.insert(id.toLongLong());
    }

    qCDebug(KWIN_EFFECTS) << "ViewOS Face Tilt reconfigured:" << m_enabled;
}

bool ViewOSFaceTiltEffect::isActive() const
{
    return m_enabled && m_tiltState.confidence > 0.1f;
}

void ViewOSFaceTiltEffect::prePaintWindow(EffectWindow* w, WindowPrePaintData& data, int time)
{
    if (!isActive() || !shouldTransformWindow(w)) {
        return;
    }

    // Mark window as transformed so KWin knows to call paintWindow with transform
    data.setTransformed();
}

void ViewOSFaceTiltEffect::paintWindow(EffectWindow* w, int mask, QRegion region, WindowPaintData& data)
{
    if (!isActive() || !shouldTransformWindow(w)) {
        return;
    }

    applyTiltTransform(w, data);
}

void ViewOSFaceTiltEffect::postPaintWindow(EffectWindow* w)
{
    // Schedule repaint for animation
    if (isActive() && shouldTransformWindow(w)) {
        w->addRepaintFull();
    }
}

void ViewOSFaceTiltEffect::prePaintScreen(ScreenPrePaintData& data, int time)
{
    // If any window is transformed, we need full screen repaint
    if (isActive()) {
        data.setTransformed();
    }
}

bool ViewOSFaceTiltEffect::shouldTransformWindow(EffectWindow* w) const
{
    if (!w || !m_enabled) {
        return false;
    }

    // Exclude by window ID
    if (m_excludedWindowIds.contains(w->windowId())) {
        return false;
    }

    // Exclude fullscreen windows
    if (w->isFullScreen()) {
        return false;
    }

    // Exclude maximized windows
    if (w->isMaximized()) {
        return false;
    }

    // Exclude dialogs, popups, menus, tooltips
    if (w->windowType() == NET::Dialog
        || w->windowType() == NET::Utility
        || w->windowType() == NET::Toolbar
        || w->windowType() == NET::Menu
        || w->windowType() == NET::Splash
        || w->windowType() == NET::DropdownMenu
        || w->windowType() == NET::PopupMenu
        || w->windowType() == NET::Tooltip
        || w->windowType() == NET::Notification) {
        return false;
    }

    // Exclude windows with skip taskbar (often overlays)
    if (w->skipTaskbar()) {
        return false;
    }

    // Exclude KWin's own effect windows
    if (w->windowClass() == "kwin") {
        return false;
    }

    // Exclude minimized/shaded
    if (w->isMinimized() || w->isShaded()) {
        return false;
    }

    // Exclude on other desktops (unless sticky)
    if (!w->isOnAllDesktops() && !effects->activeWindowOnDesktop(w)) {
        return false;
    }

    return true;
}

void ViewOSFaceTiltEffect::applyTiltTransform(EffectWindow* w, WindowPaintData& data)
{
    // Get window center in screen coordinates
    QRectF geom = w->geometry();
    QVector3D center(geom.center().x(), geom.center().y(), 0.0f);

    // Calculate tilt angles (already smoothed in readTiltData)
    float yaw = m_smoothedState.yaw;
    float pitch = m_smoothedState.pitch;

    if (m_invertYaw) yaw = -yaw;
    if (m_invertPitch) pitch = -pitch;

    // Convert to radians for Qt
    float yawRad = qDegreesToRadians(yaw);
    float pitchRad = qDegreesToRadians(pitch);

    // Set rotation origin to window center
    data.setRotationOrigin(QVector3D(center.x(), center.y(), 0.0f));

    // Apply rotations
    // Yaw = rotation around Y axis (left/right tilt)
    data.setRotationAxis(Qt::YAxis);
    data.setRotationAngle(yawRad);

    // For pitch (up/down), we need a separate transform
    // KWin only supports one rotation axis at a time in WindowPaintData
    // So we compose them manually via the transform matrix
    QMatrix4x4 transform;
    transform.translate(center);
    transform.rotate(yawRad, 0.0f, 1.0f, 0.0f);
    transform.rotate(pitchRad, 1.0f, 0.0f, 0.0f);
    transform.translate(-center);

    data.setModelViewMatrix(transform);
}

void ViewOSFaceTiltEffect::onSocketData()
{
    if (m_socketFd < 0) return;

    char buffer[1024];
    ssize_t n = read(m_socketFd, buffer, sizeof(buffer) - 1);
    if (n <= 0) {
        return;
    }

    buffer[n] = '\0';
    m_socketBuffer.append(buffer, n);

    // Process complete lines (JSON per line)
    while (true) {
        int newline = m_socketBuffer.indexOf('\n');
        if (newline < 0) break;

        QByteArray line = m_socketBuffer.left(newline);
        m_socketBuffer.remove(0, newline + 1);

        QJsonParseError error;
        QJsonDocument doc = QJsonDocument::fromJson(line, &error);
        if (error.error != QJsonParseError::NoError) {
            continue;
        }

        QJsonObject obj = doc.object();
        float yaw = obj.value("yaw").toDouble(0.0);
        float pitch = obj.value("pitch").toDouble(0.0);
        float roll = obj.value("roll").toDouble(0.0);
        float confidence = obj.value("confidence").toDouble(0.0);
        qint64 timestamp = obj.value("timestamp").toVariant().toLongLong();

        // Apply deadzone
        if (qAbs(yaw) < m_deadzone) yaw = 0.0f;
        if (qAbs(pitch) < m_deadzone) pitch = 0.0f;

        // Clamp
        yaw = qBound(-m_maxAngle, yaw, m_maxAngle);
        pitch = qBound(-m_maxAngle, pitch, m_maxAngle);

        // Smooth with exponential moving average
        m_smoothedState.yaw = m_smoothedState.yaw * (1.0f - m_smoothing) + yaw * m_smoothing;
        m_smoothedState.pitch = m_smoothedState.pitch * (1.0f - m_smoothing) + pitch * m_smoothing;
        m_smoothedState.roll = roll;
        m_smoothedState.confidence = confidence;
        m_smoothedState.timestamp = timestamp;

        m_tiltState.yaw = yaw;
        m_tiltState.pitch = pitch;
        m_tiltState.roll = roll;
        m_tiltState.confidence = confidence;
        m_tiltState.timestamp = timestamp;

        // Schedule repaint of all affected windows
        for (EffectWindow* w : effects->stackingOrder()) {
            if (shouldTransformWindow(w)) {
                w->addRepaintFull();
            }
        }
    }
}

void ViewOSFaceTiltEffect::onConfigChanged()
{
    reconfigure(ReconfigureAll);
}

} // namespace KWin

#include "viewosfacetilt.moc"