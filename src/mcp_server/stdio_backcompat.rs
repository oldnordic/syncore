use rmcp::{
    model::{
        ClientJsonRpcMessage, ClientNotification, ClientRequest, InitializedNotification,
        JsonRpcMessage,
    },
    service::{RoleServer, RxJsonRpcMessage, TxJsonRpcMessage},
    transport::{io::stdio, IntoTransport, Transport, TransportAdapterIdentity},
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Wrapper around rmcp's stdio transport that optionally tolerates missing `notifications/initialized`
pub struct BackcompatStdIo {
    backwards_compatible: bool,
}

impl BackcompatStdIo {
    pub fn new(backwards_compatible: bool) -> Self {
        Self {
            backwards_compatible,
        }
    }
}

impl IntoTransport<RoleServer, std::io::Error, TransportAdapterIdentity> for BackcompatStdIo {
    fn into_transport(self) -> impl Transport<RoleServer, Error = std::io::Error> + 'static {
        let base_transport = <(tokio::io::Stdin, tokio::io::Stdout) as IntoTransport<
            RoleServer,
            std::io::Error,
            _,
        >>::into_transport(stdio());
        BackcompatTransport::new(base_transport, self.backwards_compatible)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HandshakeStage {
    AwaitInitialize,
    AwaitInitialized,
    Ready,
}

struct BackcompatTransport<T> {
    inner: T,
    stage: Arc<Mutex<HandshakeStage>>,
    backwards_compatible: bool,
}

impl<T> BackcompatTransport<T> {
    fn new(inner: T, backwards_compatible: bool) -> Self {
        Self {
            inner,
            stage: Arc::new(Mutex::new(HandshakeStage::AwaitInitialize)),
            backwards_compatible,
        }
    }
}

impl<T> Transport<RoleServer> for BackcompatTransport<T>
where
    T: Transport<RoleServer> + Send + 'static,
{
    type Error = T::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.inner.send(item)
    }

    fn receive(
        &mut self,
    ) -> impl std::future::Future<Output = Option<RxJsonRpcMessage<RoleServer>>> + Send {
        let stage = self.stage.clone();
        let backwards = self.backwards_compatible;
        let inner_future = self.inner.receive();
        async move {
            match inner_future.await {
                Some(msg) => {
                    if backwards {
                        update_stage(&stage, &msg).await;
                    }
                    Some(msg)
                }
                None => {
                    if backwards && try_inject_initialized(&stage).await {
                        eprintln!(
                            "[BackwardsCompatible] Auto-injecting notifications/initialized notification"
                        );
                        Some(ClientJsonRpcMessage::notification(
                            ClientNotification::InitializedNotification(
                                InitializedNotification::default(),
                            ),
                        ))
                    } else {
                        None
                    }
                }
            }
        }
    }

    fn close(&mut self) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        self.inner.close()
    }
}

async fn update_stage(stage: &Arc<Mutex<HandshakeStage>>, message: &RxJsonRpcMessage<RoleServer>) {
    let mut guard = stage.lock().await;
    match message {
        JsonRpcMessage::Request(req) => {
            if matches!(req.request, ClientRequest::InitializeRequest(_)) {
                *guard = HandshakeStage::AwaitInitialized;
            }
        }
        JsonRpcMessage::Notification(notif) => {
            if matches!(notif.notification, ClientNotification::InitializedNotification(_)) {
                *guard = HandshakeStage::Ready;
            }
        }
        _ => {}
    }
}

async fn try_inject_initialized(stage: &Arc<Mutex<HandshakeStage>>) -> bool {
    let mut guard = stage.lock().await;
    if matches!(*guard, HandshakeStage::AwaitInitialized) {
        *guard = HandshakeStage::Ready;
        true
    } else {
        false
    }
}
