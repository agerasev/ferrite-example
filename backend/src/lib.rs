use ferrite::{
    entry_point, AnyVariable, Context, Downcast, ReadArrayVariable, ReadVariable, Registry,
    WriteArrayVariable, WriteVariable,
};
use futures::{
    executor::{block_on, ThreadPool},
    future::pending,
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

fn take_from_registry<V>(reg: &mut Registry, name: &str) -> Option<V>
where
    AnyVariable: Downcast<V>,
{
    reg.remove(name)?.downcast()
}

async fn async_main(exec: ThreadPool, mut ctx: Context) {
    log::info!("IOC started");

    let mut ai: WriteVariable<i32> = take_from_registry(&mut ctx.registry, "example:ai").unwrap();
    let mut ao: ReadVariable<i32> = take_from_registry(&mut ctx.registry, "example:ao").unwrap();
    let mut aai: WriteArrayVariable<i32> =
        take_from_registry(&mut ctx.registry, "example:aai").unwrap();
    let mut aao: ReadArrayVariable<i32> =
        take_from_registry(&mut ctx.registry, "example:aao").unwrap();
    let mut waveform: WriteArrayVariable<i32> =
        take_from_registry(&mut ctx.registry, "example:waveform").unwrap();
    let mut bi: WriteVariable<u32> = take_from_registry(&mut ctx.registry, "example:bi").unwrap();
    let mut bo: ReadVariable<u32> = take_from_registry(&mut ctx.registry, "example:bo").unwrap();
    let mut mbbi_direct: WriteVariable<u32> =
        take_from_registry(&mut ctx.registry, "example:mbbiDirect").unwrap();
    let mut mbbo_direct: ReadVariable<u32> =
        take_from_registry(&mut ctx.registry, "example:mbboDirect").unwrap();

    assert!(ctx.registry.is_empty());

    exec.spawn_ok(async move {
        loop {
            let value = ao.read().await;
            log::info!("ao -> ai: {}", value);
            ai.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        assert!(aao.max_len() <= aai.max_len());
        let mut buffer = vec![0; aao.max_len()];
        loop {
            let len = aao.read_to_slice(&mut buffer).await.unwrap();
            let value = &buffer[..len];
            log::info!("aao -> (aai, waveform): {:?}", value);
            aai.write_from_slice(value).await;
            waveform.write_from_slice(value).await;
        }
    });
    exec.spawn_ok(async move {
        loop {
            let value = bo.read().await;
            log::info!("bo -> bi: {}", value != 0);
            bi.write(value).await;
        }
    });
    exec.spawn_ok(async move {
        loop {
            let value = mbbo_direct.read().await;
            log::info!("mbbo_direct -> mbbi_direct: {:016b}", value);
            mbbi_direct.write(value).await;
        }
    });

    pending::<()>().await;
}
