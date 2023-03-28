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
        {
            let init = std::f64::consts::PI;
            ao.request().await.write(init).await;
            ai.request().await.write(init).await;
            log::info!("init (ao, ai): {}", init);
        }
        loop {
            let guard = ao.wait().await;
            ai.request().await.write(*guard).await;
            log::debug!("ao -> ai: {}", *guard);
            guard.accept().await;
        }
    });
    exec.spawn_ok(async move {
        assert!(aao.max_len() <= aai.max_len());
        {
            let init = 0..(aao.max_len() as i32);
            aao.request().await.write_from(init.clone()).await;
            join!(
                async { aai.request().await.write_from_iter(init.clone()).await },
                async { waveform.request().await.write_from_iter(init.clone()).await }
            );
            log::info!("init (aao, aai, waveform): {:?}", init);
        }
        loop {
            let guard = aao.wait().await;
            join!(
                async { aai.request().await.write_from_slice(&guard).await },
                async { waveform.request().await.write_from_slice(&guard).await }
            );
            log::debug!("aao -> (aai, waveform): {:?}", guard.as_slice());
            guard.accept().await;
        }
    });
    exec.spawn_ok(async move {
        {
            let init = 1;
            bo.request().await.write(init).await;
            bi.request().await.write(init).await;
            log::info!("init (bo, bi): {}", init != 0);
        }
        loop {
            let guard = bo.wait().await;
            bi.request().await.write(*guard).await;
            log::debug!("bo -> bi: {}", *guard != 0);
            guard.accept().await;
        }
    });
    exec.spawn_ok(async move {
        {
            let init = 0x7aaa_aaaa;
            longout.request().await.write(init).await;
            longin.request().await.write(init).await;
            log::info!("init (longout, longin): {}", init);
        }
        loop {
            let guard = longout.wait().await;
            longin.request().await.write(*guard).await;
            log::debug!("longout -> longin: {}", *guard);
            guard.accept().await;
        }
    });
    exec.spawn_ok(async move {
        {
            let init = 0xaaaa_aaaa;
            mbbo_direct.request().await.write(init).await;
            log::info!("init (mbbo_direct, mbbi_direct): {:032b}", init);
        }
        loop {
            let guard = mbbo_direct.wait().await;
            mbbi_direct.request().await.write(*guard).await;
            log::debug!("mbbo_direct -> mbbi_direct: {:032b}", *guard);
            guard.accept().await;
        }
    });
    exec.spawn_ok(async move {
        assert!(stringout.max_len() <= stringin.max_len());
        {
            let init = "Hello, Ferrite!";
            stringout
                .request()
                .await
                .write_from_slice(init.as_bytes())
                .await;
            stringin
                .request()
                .await
                .write_from_slice(init.as_bytes())
                .await;
            log::info!("init: (stringout, stringin): '{}'", init);
        }
        loop {
            let guard = stringout.wait().await;
            stringin.request().await.write_from_slice(&guard).await;
            log::debug!(
                "stringout -> stringin: '{}'",
                String::from_utf8_lossy(&guard)
            );
            guard.accept().await;
        }
    });

    pending::<()>().await;
}
