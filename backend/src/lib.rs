use ferrite::{entry_point, prelude::*, AnyVariable, ArrayVariable, Context, Variable};
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

    let exec = ThreadPool::builder().pool_size(2).create().unwrap();
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
        .expect(&format!("No such name: {}", name));
    let info = var.info();
    var.downcast()
        .expect(&format!("{}: Wrong type, {:?} expected", name, info))
}

async fn async_main(exec: ThreadPool, mut ctx: Context) {
    log::info!("IOC started");

    let mut ai: Variable<f64, false, true, true> = take_var(&mut ctx, "example:ai");
    let mut ao: Variable<f64, true, true, false> = take_var(&mut ctx, "example:ao");
    let mut aai: ArrayVariable<i32, false, true, true> = take_var(&mut ctx, "example:aai");
    let mut aao: ArrayVariable<i32, true, true, false> = take_var(&mut ctx, "example:aao");
    let mut waveform: ArrayVariable<i32, false, true, true> =
        take_var(&mut ctx, "example:waveform");
    let mut bi: Variable<u16, false, true, true> = take_var(&mut ctx, "example:bi");
    let mut bo: Variable<u16, true, true, false> = take_var(&mut ctx, "example:bo");
    let mut mbbi_direct: Variable<u32, false, true, true> =
        take_var(&mut ctx, "example:mbbiDirect");
    let mut mbbo_direct: Variable<u32, true, true, false> =
        take_var(&mut ctx, "example:mbboDirect");
    let mut stringin: ArrayVariable<u8, false, true, true> = take_var(&mut ctx, "example:stringin");
    let mut stringout: ArrayVariable<u8, true, true, false> =
        take_var(&mut ctx, "example:stringout");

    assert!(ctx.registry.is_empty());

    exec.spawn_ok(async move {
        let mut init = Some(std::f64::consts::PI);
        loop {
            let value = match init.take() {
                Some(init) => {
                    ao.wait().await.write(init).await;
                    init
                }
                None => ao.wait().await.read().await,
            };
            log::info!("ao -> ai: {}", value);
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
                    let mut value = aao.wait().await.write();
                    value.extend(init);
                    buffer.extend_from_slice(&value);
                    value.commit().await;
                }
                None => {
                    let value = aao.wait().await.read();
                    buffer.extend_from_slice(&value);
                    value.close().await;
                }
            };
            log::info!("aao -> (aai, waveform): {:?}", buffer);
            let mut ovals = (
                aai.request().await.write(),
                waveform.request().await.write(),
            );
            ovals.0.extend_from_slice(&buffer);
            ovals.1.extend_from_slice(&buffer);
            join!(ovals.0.commit(), ovals.1.commit());
        }
    });
    exec.spawn_ok(async move {
        let mut init = Some(1);
        loop {
            let value = match init.take() {
                Some(init) => {
                    bo.wait().await.write(init).await;
                    init
                }
                None => bo.wait().await.read().await,
            };
            log::info!("bo -> bi: {}", value != 0);
            bi.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        let mut init = Some(0xaaaa);
        loop {
            let value = match init.take() {
                Some(init) => {
                    mbbo_direct.wait().await.write(init).await;
                    init
                }
                None => mbbo_direct.wait().await.read().await,
            };
            log::info!("mbbo_direct -> mbbi_direct: {:016b}", value);
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
                    let mut value = stringout.wait().await.write();
                    value.extend_from_slice(init.as_bytes());
                    buffer.extend_from_slice(&value);
                    value.commit().await;
                }
                None => {
                    let value = stringout.wait().await.read();
                    buffer.extend_from_slice(&value);
                    value.close().await;
                }
            };
            log::info!(
                "stringout -> stringin: '{}'",
                String::from_utf8_lossy(&buffer)
            );
            let mut oval = stringin.request().await.write();
            oval.extend_from_slice(&buffer);
            oval.commit().await;
        }
    });

    pending::<()>().await;
}
