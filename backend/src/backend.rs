use core::time;
use std::{sync::{Arc}, thread};
use tokio::sync::{oneshot, Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
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

    type ExecuteStreamStream = ReceiverStream<Result<ExecuteStreamResponse, Status>>;

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

    async fn setup_stream(
        &self,
        request: Request<SetupRequest>,
    ) -> Result<Response<SetupResponse>, Status> {
        let procedure_stream = request.into_inner().procedure;

        if procedure_stream.is_empty() {
            return Err(Status::invalid_argument("procedure string can not be empty"));
        }

        let mut guard = self.inner.lock().await;

        match &guard.state {
            State::Terminated => {
                return Err(Status::failed_precondition("backend terminated",));
            }
            State::Ready { procedure: prev } => {
                warn!(previous = %prev, new = %procedure_stream, "re-configure procedure");
            }
            State::Idle => {}
        }
        info!(procedure_stream = %procedure_stream, "setting up procedure");
        guard.state = State::Ready { procedure: procedure_stream.clone() };

        Ok(Response::new(SetupResponse {
            success: true,
            message: format!("procedure '{}' is ready", procedure_stream),
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

        let result = run_procedure(&procedure);
 
        info!(procedure = %procedure, "executing procedure");
 
        Ok(Response::new(ExecuteResponse {
            success: true,
            message: format!("'{:?}'", result.await),
            // message: format!("procedure '{}' executed successfully", procedure),
        }))
    }
 
    // TODO stream execute
    async fn execute_stream(
        &self,
        _request: Request<ExecuteStreamRequest>,
    ) -> Result<Response<Self::ExecuteStreamStream>, Status> {
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

        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            // Simulate streaming work (replace with your actual logic)
            for i in 0..10 {
                let chunk = ExecuteStreamResponse {
                    payload: Some(execute_stream_response::Payload::Chunk(
                        format!("Output chunk {} for: {}", i, procedure),
                    )),
                    sequence: i,
                };

                // If receiver dropped (client disconnected), stop processing
                if tx.send(Ok(chunk)).await.is_err() {
                    println!("Client disconnected, stopping stream");
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }

            // Send completion signal
            let _ = tx
                .send(Ok(ExecuteStreamResponse {
                    payload: Some(execute_stream_response::Payload::Done(true)),
                    sequence: -1,
                }));
        });

        Ok(Response::new(ReceiverStream::new(rx)))

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
        "hello world" => {
            thread::sleep(time::Duration::from_secs(5));
            "Hello World!".to_string()},
        other   => format!("Ran unknown procedure '{}' – no specific handler registered.", other),
    }
}
