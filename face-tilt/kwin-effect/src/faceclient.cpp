#include "faceclient.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QTimer>

FaceClient::FaceClient(QObject* parent)
    : QObject(parent)
    , m_socket(new QLocalSocket(this))
{
    connect(m_socket, &QLocalSocket::readyRead, this, &FaceClient::onReadyRead);
    connect(m_socket, &QLocalSocket::connected, this, &FaceClient::onConnected);
    connect(m_socket, &QLocalSocket::disconnected, this, &FaceClient::onDisconnected);
    connect(m_socket, QOverload<QLocalSocket::LocalSocketError>::of(&QLocalSocket::errorOccurred),
            this, &FaceClient::onError);
}

FaceClient::~FaceClient()
{
    disconnect();
}

bool FaceClient::connectToDaemon(const QString& socketPath)
{
    if (m_socket->state() == QLocalSocket::ConnectedState) {
        return true;
    }

    m_socket->connectToServer(socketPath);
    return m_socket->waitForConnected(1000);
}

void FaceClient::disconnect()
{
    if (m_socket->state() != QLocalSocket::UnconnectedState) {
        m_socket->disconnectFromServer();
        m_socket->waitForDisconnected(1000);
    }
}

void FaceClient::onReadyRead()
{
    m_buffer.append(m_socket->readAll());

    while (true) {
        int newline = m_buffer.indexOf('\n');
        if (newline < 0) break;

        QByteArray line = m_buffer.left(newline);
        m_buffer.remove(0, newline + 1);

        QJsonParseError error;
        QJsonDocument doc = QJsonDocument::fromJson(line, &error);
        if (error.error != QJsonParseError::NoError) {
            continue;
        }

        QJsonObject obj = doc.object();
        TiltData data;
        data.yaw = obj.value("yaw").toDouble(0.0);
        data.pitch = obj.value("pitch").toDouble(0.0);
        data.roll = obj.value("roll").toDouble(0.0);
        data.confidence = obj.value("confidence").toDouble(0.0);
        data.timestamp = obj.value("timestamp").toVariant().toLongLong();

        emit tiltDataReceived(data);
    }
}

void FaceClient::onConnected()
{
    emit connectionChanged(true);
}

void FaceClient::onDisconnected()
{
    emit connectionChanged(false);
    // Auto-reconnect after 5 seconds
    QTimer::singleShot(5000, this, [this]() {
        connectToDaemon();
    });
}

void FaceClient::onError(QLocalSocket::LocalSocketError error)
{
    Q_UNUSED(error);
    emit errorOccurred(m_socket->errorString());
}