use ferrite::{entry_point, Context, Downcast, TypedVariable as Variable, Variable as AnyVariable};
use futures::{
    executor::{block_on, ThreadPool},
    future::pending,
    join,
};
use macro_rules_attribute::apply;

/// *Export symbols being called from IOC.*
pub use ferrite::export;

#[apply(entry_point)]
fn app_main(mut ctx: Context) {
    use env_logger::Env;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let exec = ThreadPool::builder().create().unwrap();
    block_on(async_main(exec, ctx));
}

fn take_var<V>(ctx: &mut Context, name: &str) -> V
where
    AnyVariable: Downcast<V>,
{
    log::debug!("take: {}", name);
    let var = ctx
        .registry
        .remove(name)
        .unwrap_or_else(|| panic!("No such name: {}", name));
    let info = var.info();
    var.downcast()
        .unwrap_or_else(|| panic!("{}: Wrong type, {:?} expected", name, info))
}

async fn async_main(exec: ThreadPool, mut ctx: Context) {
    log::info!("IOC started");

    let mut ai: Variable<f64> = take_var(&mut ctx, "example:ai");
    let mut ao: Variable<f64> = take_var(&mut ctx, "example:ao");
    let mut aai: Variable<[i32]> = take_var(&mut ctx, "example:aai");
    let mut aao: Variable<[i32]> = take_var(&mut ctx, "example:aao");
    let mut waveform: Variable<[i32]> = take_var(&mut ctx, "example:waveform");
    let mut bi: Variable<u16> = take_var(&mut ctx, "example:bi");
    let mut bo: Variable<u16> = take_var(&mut ctx, "example:bo");
    let mut longin: Variable<i32> = take_var(&mut ctx, "example:longin");
    let mut longout: Variable<i32> = take_var(&mut ctx, "example:longout");
    let mut mbbi_direct: Variable<u32> = take_var(&mut ctx, "example:mbbiDirect");
    let mut mbbo_direct: Variable<u32> = take_var(&mut ctx, "example:mbboDirect");
    let mut stringin: Variable<[u8]> = take_var(&mut ctx, "example:stringin");
    let mut stringout: Variable<[u8]> = take_var(&mut ctx, "example:stringout");

    assert!(ctx.registry.is_empty());

    exec.spawn_ok(async move {
        let mut init = Some(std::f64::consts::PI);
        loop {
            let value = match init.take() {
                Some(init) => {
                    ao.request().await.write(init).await;
                    init
                }
                None => ao.wait().await.read().await,
            };
            log::debug!("ao -> ai: {}", value);
            ai.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        assert!(aao.max_len() <= aai.max_len());
        let mut buffer = Vec::with_capacity(aao.max_len());
        let mut init = Some(0..(aao.max_len() as i32));
        loop {
            buffer.clear();
            match init.take() {
                Some(init) => {
                    aao.request().await.write_from(init.clone()).await;
                    buffer.extend(init);
                }
                None => {
                    aao.wait().await.read_to_vec(&mut buffer).await;
                }
            };
            log::debug!("aao -> (aai, waveform): {:?}", buffer);
            join!(
                async { aai.request().await.write_from_slice(&buffer).await },
                async { waveform.request().await.write_from_slice(&buffer).await }
            );
        }
    });
    exec.spawn_ok(async move {
        let mut init = Some(1);
        loop {
            let value = match init.take() {
                Some(init) => {
                    bo.request().await.write(init).await;
                    init
                }
                None => bo.wait().await.read().await,
            };
            log::debug!("bo -> bi: {}", value != 0);
            bi.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        let mut init = Some(0x7aaa_aaaa);
        loop {
            let value = match init.take() {
                Some(init) => {
                    longout.request().await.write(init).await;
                    init
                }
                None => longout.wait().await.read().await,
            };
            log::debug!("longout -> longin: {}", value);
            longin.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        let mut init = Some(0xaaaaaaaa);
        loop {
            let value = match init.take() {
                Some(init) => {
                    mbbo_direct.request().await.write(init).await;
                    init
                }
                None => mbbo_direct.wait().await.read().await,
            };
            log::debug!("mbbo_direct -> mbbi_direct: {:032b}", value);
            mbbi_direct.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        assert!(stringout.max_len() <= stringin.max_len());
        let mut buffer = Vec::with_capacity(stringout.max_len());
        let mut init = Some("Hello, Ferrite!");
        loop {
            buffer.clear();
            match init.take() {
                Some(init) => {
                    stringout
                        .request()
                        .await
                        .write_from_slice(init.as_bytes())
                        .await;
                    buffer.extend_from_slice(init.as_bytes());
                }
                None => {
                    stringout.wait().await.read_to_vec(&mut buffer).await;
                }
            };
            log::debug!(
                "stringout -> stringin: '{}'",
                String::from_utf8_lossy(&buffer)
            );
            stringin.request().await.write_from_slice(&buffer).await;
        }
    });

    pending::<()>().await;
}
