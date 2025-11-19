use dioxus::prelude::*;

#[derive(Clone)]
pub struct UseAbortControllerReturn {
    pub abort_request: Signal<Option<Box<dyn Fn()>>>,
}

impl UseAbortControllerReturn {
    #[cfg(target_arch = "wasm32")]
    pub async fn perform_abort_request(&self, request_id: &str) {
        use reqwasm::http::Request;

        let url = format!("http://localhost:3401/api/abort/{}", request_id);

        match Request::post(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!("Abort request sent for request_id: {}", request_id);
            }
            Err(e) => {
                tracing::error!("Failed to abort request: {:?}", e);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn perform_abort_request(&self, request_id: &str) {
        // Server-side: would need a server function to abort
        tracing::warn!("Abort request not implemented for server-side: {}", request_id);
    }

    pub async fn abort_request_with_cleanup(
        &self,
        request_id: Option<String>,
        is_loading: bool,
        on_abort_complete: impl Fn(),
    ) {
        if request_id.is_none() || !is_loading {
            return;
        }

        let req_id = request_id.unwrap();
        self.perform_abort_request(&req_id).await;
        on_abort_complete();
    }

    pub fn create_abort_handler(&self, request_id: String) -> impl Fn() {
        let controller = self.clone();
        move || {
            let controller_clone = controller.clone();
            let request_id_clone = request_id.clone();
            spawn(async move {
                controller_clone.perform_abort_request(&request_id_clone).await;
            });
        }
    }
}

pub fn use_abort_controller() -> UseAbortControllerReturn {
    let abort_request = use_signal(|| None::<Box<dyn Fn()>>);

    UseAbortControllerReturn {
        abort_request,
    }
}
