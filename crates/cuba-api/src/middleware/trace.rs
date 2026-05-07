use axum::{extract::Request, middleware::Next, response::Response};
use cuba_shared::response::extract_trace_id;

/// 自动注入 x-request-id 到响应的中间件
pub async fn trace_id_middleware(req: Request, next: Next) -> Response {
    let trace_id = extract_trace_id(&req);

    let mut response = next.run(req).await;

    // 把 trace_id 存入 response extensions，方便后续自定义响应处理
    response.extensions_mut().insert(trace_id);

    response
}
