// src/proxy/acme_service.rs
use async_trait::async_trait;
use pingora::http::ResponseHeader;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora_error::{Error, ErrorType, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub type AcmeChallengeState = Arc<RwLock<HashMap<String, String>>>;

pub struct AcmeChallengeService {
    pub challenge_state: AcmeChallengeState,
}

#[async_trait]
impl ProxyHttp for AcmeChallengeService {
    type CTX = ();
    fn new_ctx(&self) {
        
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        let path = session.req_header().uri.path();
        if let Some(token) = path.strip_prefix("/.well-known/acme-challenge/") {
            // 在独立作用域内获取锁、克隆数据并立即释放锁
            let key_auth = {
                let state = self.challenge_state.read().unwrap();
                state.get(token).cloned() // 克隆数据
            }; // <- `state` 在这里被丢弃，锁被释放

            if let Some(key_auth_value) = key_auth {
                let mut resp = ResponseHeader::build(200, None)?;
                resp.insert_header("Content-Type", "text/plain")?;
                // 必须将 key_auth_value 拷贝为 'static 生命周期的数据
                let body = key_auth_value.clone().into_bytes();
                session.write_response_header(Box::new(resp), false).await?;
                session.write_response_body(Some(body.into()), true).await?;
            } else {
                session.respond_error(404).await?;
            }
            return Ok(true); // 请求已处理，停止处理。
        }
        Ok(false) // 如果路径不匹配，则继续到下一个服务。
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // This service does not proxy requests, so this should never be called.
        let e = Error::new(ErrorType::InternalError);
        Err(e)
    }
}
