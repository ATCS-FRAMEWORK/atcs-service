use crate::store::Store;
use std::sync::Arc;

pub struct Learn { pub store: Arc<Store>, pub falhas_consecutivas: Arc<tokio::sync::Mutex<u32>> }

impl Learn {
    pub fn new(store: Arc<Store>) -> Self {
        Self { store, falhas_consecutivas: Arc::new(tokio::sync::Mutex::new(0)) }
    }
    pub async fn analisar_falhas_e_aprender(&self, sucesso: bool) -> String {
        let mut falhas = self.falhas_consecutivas.lock().await;
        if sucesso { *falhas = 0; return "normal".to_string(); }
        *falhas += 1;
        if *falhas >= 3 {
            self.store.write_event("LEARN", "Modo de seguranca ativado.").await;
            *falhas = 0; return "modo_seguranca".to_string();
        }
        "alerta".to_string()
    }
    pub async fn correlacionar_historico(&self) {
        if let Some(padrao) = self.store.buscar_padrao_correlacao().await {
            self.store.write_event("LEARN_MEMORIA", &padrao).await;
        }
    }
    pub async fn registrar_sucesso(&self, modulo: &str) {
        self.store.write_event("SUCCESS", &format!("Modulo {} OK.", modulo)).await;
    }
}
