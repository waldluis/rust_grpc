use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tonic::{Request, Response, Status};
use tracing::{info, warn};


tonic::include_proto!("backend");


use self::backend_server::Backend;

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Idle,
    Ready {procedure: String},
    Terminated,
}

pub struct BackendState {
    pub state: State,
}

impl BackendState {
    pub fn new() -> Self {
        Self { state: State::Idle}
    }
}

// Service
pub struct BackendService {
    inner: Arc<Mutex<BackendState>>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl BackendService {
    pub fn new(inner: Arc<Mutex<BackendState>>, shutdown_tx: oneshot::Sender<()>) -> Self {
        Self {
            inner, 
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        }
    }
}

#[tonic::async_trait]
impl Backend for BackendService {
    async fn setup(
        &self,
        request: Request<SetupRequest>,
    ) -> Result<Response<SetupResponse>, Status> {
        let procedure = request.into_inner().procedure;

        if procedure.is_empty() {
            return Err(Status::invalid_argument("procedure string can not be empty"));
        }

        let mut guard = self.inner.lock().await;

        match &guard.state {
            State::Terminated => {
                return Err(Status::failed_precondition("backend terminated",));
            }
            State::Ready { procedure: prev } => {
                warn!(previous = %prev, new = %procedure, "re-configure procedure");
            }
            State::Idle => {}
        }
        info!(procedure = %procedure, "setting up procedure");
        guard.state = State::Ready { procedure: procedure.clone() };

        Ok(Response::new(SetupResponse {
            success: true,
            message: format!("procedure '{}' is ready", procedure),
        }))
    }

    async fn execute(
        &self,
        _request: Request<ExecuteRequest>,
    ) -> Result<Response<ExecuteResponse>, Status> {
        let guard = self.inner.lock().await;
 
        let procedure = match &guard.state {
            State::Ready { procedure } => procedure.clone(),
            State::Idle => {
                return Err(Status::failed_precondition(
                    "no procedure configured; call Setup first",
                ));
            }
            State::Terminated => {
                return Err(Status::failed_precondition(
                    "backend is terminated",
                ));
            }
        };

        drop(guard);
 
        info!(procedure = %procedure, "executing procedure");
 
        Ok(Response::new(ExecuteResponse {
            success: true,
            message: format!("procedure '{}' executed successfully", procedure),
        }))
    }
 
    async fn terminate(
        &self,
        _request: Request<TerminateRequest>,
    ) -> Result<Response<TerminateResponse>, Status> {
        let mut guard = self.inner.lock().await;
 
        if guard.state == State::Terminated {
            return Err(Status::failed_precondition("already terminated"));
        }
 
        info!("terminating backend");
        guard.state = State::Terminated;
 
        // Signal the server to begin its graceful shutdown.
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
 
        Ok(Response::new(TerminateResponse {
            success: true,
            message: "backend terminated; shutting down".to_string(),
        }))
    }
}

async fn run_procedure(procedure: &str) -> String {
    match procedure {
        "echo" => "Echo procedure: all systems nominal.".to_string(),
        "ping" => "Pong!".to_string(),
        other   => format!("Ran unknown procedure '{}' – no specific handler registered.", other),
    }
}
