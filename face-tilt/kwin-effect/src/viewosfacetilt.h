/*
 * SPDX-FileCopyrightText: 2026 ViewOS Project
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * KWin effect that rotates windows to face the viewer's head.
 */

#pragma once

#include <effect/effect.h>
#include <effect/effectwindow.h>
#include <effect/effecthandler.h>

#include "faceclient.h"

#include <QSet>

namespace KWin
{

/**
 * Per-window head-facing rotation.
 *
 * The effect receives the viewer's head position from viewos-face-daemon as a
 * position on the screen, then rotates each eligible window about its own
 * centre so that the window's normal points at the head.
 *
 * Windows that would be wrong or unsafe to rotate are excluded; see
 * isEligible() for the list and the reasoning.
 */
class ViewOSFaceTiltEffect : public Effect
{
    Q_OBJECT

public:
    ViewOSFaceTiltEffect(QObject *parent = nullptr);
    ~ViewOSFaceTiltEffect() override;

    void reconfigure(Effect::ReconfigureFlags flags) override;
    bool isActive() const override;

    void prePaintScreen(ScreenPrePaintData &data, std::chrono::milliseconds presentTime) override;
    void paintScreen(const RenderTarget &renderTarget, const RenderViewport &viewport, int mask,
                     const QRegion &region, Output *screen) override;
    void postPaintScreen() override;

    void prePaintWindow(EffectWindow *w, WindowPrePaintData &data,
                        std::chrono::milliseconds presentTime) override;
    void paintWindow(const RenderTarget &renderTarget, const RenderViewport &viewport, EffectWindow *w,
                     int mask, QRegion region, WindowPaintData &data) override;

    /**
     * Whether this window should be rotated.
     *
     * Excluded, with reasoning:
     *  - fullscreen windows, because rotating them would letterbox or clip
     *    video and would move the position of on-screen controls out from
     *    under the pointer;
     *  - windows whose frame already fills their output, which is how a
     *    maximised window presents on both X11 and Wayland, for the same
     *    reason as fullscreen;
     *  - dialogs, popups, menus, tooltips, notifications, docks, desktops and
     *    other special windows, because a tilted menu detaches visually from
     *    the window it belongs to and makes it hard to read;
     *  - the lock screen and screen saver, which must not move at all while
     *    the session is locked;
     *  - input methods and on-screen displays, which have to stay under the
     *    cursor;
     *  - anything hidden, minimised, or not on the current desktop, which
     *    should not be painted transformed at all.
     */
    bool isEligible(EffectWindow *w) const;

private:
    void loadConfig();
    void onPoseReceived(const ViewOS::HeadPose &pose);
    void onConnectionChanged(bool connected);
    void resetToNeutral();

    /**
     * Rotation that makes the window centred at @p centreFraction face the
     * head, as an axis-angle pair in the maths frame (+X right, +Y up).
     *
     * This is the only place where the socket's screen coordinate convention
     * (Y down) is converted to the maths convention (Y up).
     */
    void targetRotation(const QPointF &centreFraction, QVector3D *axis, float *angle) const;

    /** Smoothing state, per window, keyed by internal window id. */
    struct Smoothed {
        QVector3D axis{0.0f, 0.0f, 1.0f};
        float angle = 0.0f;
        bool initialised = false;
    };

    ViewOS::FaceClient *m_client;

    bool m_enabled = false;
    bool m_havePose = false;
    ViewOS::HeadPose m_pose;

    /**
     * At most this angle, in degrees, is ever applied. Kept small because the
     * effect forces a full-screen repaint every time it runs; see
     * docs/face-tilt.md.
     */
    float m_maxAngleDeg = 10.0f;
    float m_minConfidence = 0.5f;
    /** 0 = no smoothing, 1 = never moves. Higher means laggier but steadier. */
    float m_smoothing = 0.25f;
    /** Multiplier on the computed angle. Allows under- or over-driving. */
    float m_gain = 1.0f;

    bool m_invertYaw = false;
    bool m_invertPitch = false;

    /** Window ids the user has explicitly excluded. */
    QSet<QUuid> m_excludedWindows;

    QHash<QUuid, Smoothed> m_smoothed;

    /**
     * Set when new data has arrived since the last repaint, so that
     * postPaintScreen() knows to ask for another frame. This is what bounds the
     * effect to the daemon's publish rate: with no new data, no repaint is
     * requested and the compositor goes idle.
     */
    bool m_needsAnotherFrame = false;
};

} // namespace KWin