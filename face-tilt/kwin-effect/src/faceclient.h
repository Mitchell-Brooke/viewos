#ifndef FACECLIENT_H
#define FACECLIENT_H

#include <QObject>
#include <QLocalSocket>
#include <QJsonObject>

class FaceClient : public QObject
{
    Q_OBJECT

public:
    explicit FaceClient(QObject* parent = nullptr);
    ~FaceClient();

    bool connectToDaemon(const QString& socketPath = QStringLiteral("/run/viewos/face-tilt.sock"));
    void disconnect();

    struct TiltData {
        float yaw = 0.0f;
        float pitch = 0.0f;
        float roll = 0.0f;
        float confidence = 0.0f;
        qint64 timestamp = 0;
    };

signals:
    void tiltDataReceived(const TiltData& data);
    void connectionChanged(bool connected);
    void errorOccurred(const QString& message);

private slots:
    void onReadyRead();
    void onConnected();
    void onDisconnected();
    void onError(QLocalSocket::LocalSocketError error);

private:
    QLocalSocket* m_socket = nullptr;
    QByteArray m_buffer;
};

#endif // FACECLIENT_H