#ifndef VIEWOSFACETILT_H
#define VIEWOSFACETILT_H

#include <kwin/effect.h>
#include <kwin/effectwindow.h>
#include <kwin/windowpaintdata.h>

#include <QObject>
#include <QSocketNotifier>
#include <QTimer>
#include <QJsonDocument>
#include <QJsonObject>

namespace KWin
{

class ViewOSFaceTiltEffect : public Effect
{
    Q_OBJECT

public:
    explicit ViewOSFaceTiltEffect();
    ~ViewOSFaceTiltEffect() override;

    // Effect interface
    void reconfigure(ReconfigureFlags flags) override;
    bool isActive() const override;

    // Window painting hooks
    void prePaintWindow(EffectWindow* w, WindowPrePaintData& data, int time) override;
    void paintWindow(EffectWindow* w, int mask, QRegion region, WindowPaintData& data) override;
    void postPaintWindow(EffectWindow* w) override;

    // Screen painting (for full-screen transforms if needed)
    void prePaintScreen(ScreenPrePaintData& data, int time) override;

private slots:
    void onSocketData();
    void onConfigChanged();

private:
    // Exclusion logic
    bool shouldTransformWindow(EffectWindow* w) const;

    // Apply tilt transform to window paint data
    void applyTiltTransform(EffectWindow* w, WindowPaintData& data);

    // Read latest tilt data from socket
    void readTiltData();

    // Socket connection to face daemon
    int m_socketFd = -1;
    QSocketNotifier* m_socketNotifier = nullptr;
    QByteArray m_socketBuffer;

    // Current tilt state
    struct TiltState {
        float yaw = 0.0f;       // degrees, rotation around Y
        float pitch = 0.0f;     // degrees, rotation around X
        float roll = 0.0f;      // degrees, rotation around Z
        float confidence = 0.0f;
        qint64 timestamp = 0;
    } m_tiltState;

    // Smoothed tilt state
    struct TiltState m_smoothedState;

    // Configuration
    float m_maxAngle = 12.0f;
    float m_deadzone = 2.0f;
    float m_smoothing = 0.15f;
    bool m_invertYaw = false;
    bool m_invertPitch = true;
    bool m_enabled = false;

    // Excluded window types
    QSet<qint64> m_excludedWindowIds;

    // Timer for config reload
    QTimer* m_configTimer = nullptr;
};

} // namespace KWin

#endif // VIEWOSFACETILT_H