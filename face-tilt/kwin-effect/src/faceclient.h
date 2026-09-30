/*
 * SPDX-FileCopyrightText: 2026 ViewOS Project
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Client for the viewos-face-daemon Unix socket.
 */

#pragma once

#include <QLocalSocket>
#include <QObject>

#include <cstdint>

namespace ViewOS
{

/**
 * Head position as published by viewos-face-daemon.
 *
 * See face-tilt/shared/protocol.md for the wire format. Coordinates are
 * screen-relative fractions so that the effect never needs to know the
 * physical size of the display.
 */
struct HeadPose {
    double hx = 0.5; ///< fraction of screen width, 0 = left edge
    double hy = 0.5; ///< fraction of screen height, 0 = top edge
    double hz = 600.0; ///< distance from screen surface, in millimetres
    double roll = 0.0; ///< in-plane roll in degrees (currently unused)
    double confidence = 0.0; ///< 0.0 = no face detected
    std::int64_t timestamp = 0; ///< ms since Unix epoch

    bool isValid(double minConfidence) const
    {
        return confidence >= minConfidence;
    }
};

/**
 * Reads head pose updates from the daemon and re-emits them as a signal.
 *
 * The client never writes to the socket. It reconnects with a bounded backoff
 * so that a daemon which is restarted repeatedly (for example while the user is
 * changing settings) does not cause a reconnect storm.
 */
class FaceClient : public QObject
{
    Q_OBJECT

public:
    explicit FaceClient(QObject *parent = nullptr);
    ~FaceClient() override;

    /**
     * The protocol version this build understands. A daemon reporting a
     * different version is ignored.
     */
    static constexpr int protocolVersion = 1;

    void setSocketPath(const QString &path);
    QString socketPath() const
    {
        return m_socketPath;
    }

    bool isConnected() const;

    /**
     * Force an immediate reconnect attempt, resetting the backoff.
     * Used when the effect is enabled by the user.
     */
    void reconnectNow();

Q_SIGNALS:
    /** A new head pose was received. */
    void poseReceived(const ViewOS::HeadPose &pose);

    /** Connection state changed. */
    void connectionChanged(bool connected);

private:
    void connectToDaemon();
    void scheduleReconnect();
    void onReadyRead();
    void onConnected();
    void onDisconnected();

    QLocalSocket *m_socket;
    QString m_socketPath;
    QByteArray m_buffer;

    int m_reconnectDelayMs = 250;
    static constexpr int m_maxReconnectDelayMs = 5000;
};

} // namespace ViewOS
