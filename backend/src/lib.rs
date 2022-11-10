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
        ao.wait().await.write(std::f64::consts::PI).await;
        loop {
            let guard = ao.wait().await;
            let value = guard.read().await;
            log::info!("ao -> ai: {}", value);
            ai.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        assert!(aao.max_len() <= aai.max_len());
        let mut val = aao.wait().await.write();
        val.extend(0..10);
        val.commit().await;
        loop {
            let ival = aao.wait().await.read();
            log::info!("aao -> (aai, waveform): {:?}", &*ival);
            let mut ovals = (
                aai.request().await.write(),
                waveform.request().await.write(),
            );
            ovals.0.extend_from_slice(&*ival);
            ovals.1.extend_from_slice(&*ival);
            join!(ovals.0.commit(), ovals.1.commit(), ival.close());
        }
    });
    exec.spawn_ok(async move {
        bo.wait().await.write(1).await;
        loop {
            let value = bo.wait().await.read().await;
            log::info!("bo -> bi: {}", value != 0);
            bi.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        mbbo_direct.wait().await.write(0xffff).await;
        loop {
            let value = mbbo_direct.wait().await.read().await;
            log::info!("mbbo_direct -> mbbi_direct: {:016b}", value);
            mbbi_direct.request().await.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        assert!(stringout.max_len() <= stringin.max_len());
        let mut val = stringin.wait().await.write();
        val.extend_from_slice("hello".as_bytes());
        val.commit().await;
        loop {
            let ival = stringout.wait().await.read();
            log::info!(
                "stringout -> stringin: '{}'",
                String::from_utf8_lossy(&*ival)
            );
            let mut oval = stringin.request().await.write();
            oval.extend_from_slice(&ival);
            join!(oval.commit(), ival.close());
        }
    });

    pending::<()>().await;
}
