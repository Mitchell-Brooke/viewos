/*
 * SPDX-FileCopyrightText: 2026 ViewOS Project
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#include "faceclient.h"

#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QTimer>

#include <cmath>

namespace ViewOS
{

namespace
{
/**
 * Guard against a misbehaving or hostile peer making us allocate an unbounded
 * amount of memory by never sending a newline.
 */
constexpr int maxBufferBytes = 64 * 1024;
}

FaceClient::FaceClient(QObject *parent)
    : QObject(parent)
    , m_socket(new QLocalSocket(this))
    , m_socketPath(QDir::temp().filePath(QStringLiteral("viewos-face-tilt.sock")))
{
    connect(m_socket, &QLocalSocket::readyRead, this, &FaceClient::onReadyRead);
    connect(m_socket, &QLocalSocket::connected, this, &FaceClient::onConnected);
    connect(m_socket, &QLocalSocket::disconnected, this, &FaceClient::onDisconnected);
}

FaceClient::~FaceClient()
{
    m_socket->abort();
}

void FaceClient::setSocketPath(const QString &path)
{
    if (m_socketPath == path) {
        return;
    }
    m_socketPath = path;
    m_socket->abort();
    m_buffer.clear();
    m_reconnectDelayMs = 250;
    connectToDaemon();
}

bool FaceClient::isConnected() const
{
    return m_socket->state() == QLocalSocket::ConnectedState;
}

void FaceClient::reconnectNow()
{
    m_socket->abort();
    m_reconnectDelayMs = 250;
    connectToDaemon();
}

void FaceClient::connectToDaemon()
{
    if (m_socket->state() != QLocalSocket::UnconnectedState) {
        return;
    }
    m_socket->connectToServer(m_socketPath);
}

void FaceClient::scheduleReconnect()
{
    // Exponential backoff, capped. Reset whenever a connection succeeds so that
    // a daemon which is restarted once an hour does not end up waiting five
    // seconds after the first failure of the next outage.
    QTimer::singleShot(m_reconnectDelayMs, this, &FaceClient::connectToDaemon);
    m_reconnectDelayMs = qMin(m_reconnectDelayMs * 2, m_maxReconnectDelayMs);
}

void FaceClient::onConnected()
{
    m_reconnectDelayMs = 250;
    Q_EMIT connectionChanged(true);
}

void FaceClient::onDisconnected()
{
    Q_EMIT connectionChanged(false);
    scheduleReconnect();
}

void FaceClient::onReadyRead()
{
    m_buffer.append(m_socket->readAll());

    if (m_buffer.size() > maxBufferBytes) {
        // The stream is not newline-delimited, so it cannot be resynchronised
        // safely. Dropping everything and reconnecting is the only safe
        // response.
        m_buffer.clear();
        m_socket->abort();
        return;
    }

    qsizetype newline;
    while ((newline = m_buffer.indexOf('\n')) >= 0) {
        const QByteArray line = m_buffer.left(newline);
        m_buffer.remove(0, newline + 1);

        QJsonParseError error{};
        const QJsonDocument doc = QJsonDocument::fromJson(line, &error);
        if (error.error != QJsonParseError::NoError || !doc.isObject()) {
            continue;
        }

        const QJsonObject obj = doc.object();

        const int version = obj.value(QStringLiteral("v")).toInt(-1);
        if (version != protocolVersion) {
            // Refuse to interpret a protocol we do not understand rather than
            // silently mis-rendering every window.
            continue;
        }

        HeadPose pose;
        pose.hx = obj.value(QStringLiteral("hx")).toDouble(0.5);
        pose.hy = obj.value(QStringLiteral("hy")).toDouble(0.5);
        pose.hz = obj.value(QStringLiteral("hz")).toDouble(600.0);
        pose.roll = obj.value(QStringLiteral("roll")).toDouble(0.0);
        pose.confidence = obj.value(QStringLiteral("conf")).toDouble(0.0);
        pose.timestamp = static_cast<std::int64_t>(
            static_cast<double>(obj.value(QStringLiteral("t")).toDouble(0.0)));

        if (!std::isfinite(pose.hx) || !std::isfinite(pose.hy) || !std::isfinite(pose.hz)
            || !std::isfinite(pose.confidence)) {
            continue;
        }

        Q_EMIT poseReceived(pose);
    }
}

} // namespace ViewOS

#include "faceclient.moc"
