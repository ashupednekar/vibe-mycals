use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    mycals_api::handle(req, env, ctx).await
}
