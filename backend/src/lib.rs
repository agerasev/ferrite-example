use async_std::net::TcpStream;
use ferrite::{entry_point, prelude::*, AnyVariable, ArrayVariable, Context, Variable};
use flatty::{
    portable::{le::*, NativeCast},
    vec::FromIterator,
};
use flatty_io::{AsyncReader as MsgReader, AsyncWriter as MsgWriter};
use futures::{
    executor::{block_on, ThreadPool},
    future::pending,
};
use macro_rules_attribute::apply;
use protocol::*;

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
    let mut aai: ArrayVariable<i32> = take_var(&mut ctx, "example:aai");
    let mut aao: ArrayVariable<i32> = take_var(&mut ctx, "example:aao");
    let mut waveform: ArrayVariable<i32> = take_var(&mut ctx, "example:waveform");
    let mut bi: Variable<u16> = take_var(&mut ctx, "example:bi");
    let mut bo: Variable<u16> = take_var(&mut ctx, "example:bo");
    let mut mbbi_direct: Variable<u32> = take_var(&mut ctx, "example:mbbiDirect");
    let mut mbbo_direct: Variable<u32> = take_var(&mut ctx, "example:mbboDirect");
    let mut stringin: ArrayVariable<u8> = take_var(&mut ctx, "example:stringin");
    let mut stringout: ArrayVariable<u8> = take_var(&mut ctx, "example:stringout");

    assert!(ctx.registry.is_empty());

    let max_msg_size: usize = 259;
    let stream = TcpStream::connect("127.0.0.1:4884").await.unwrap();
    let mut reader = MsgReader::<InMsg, _>::new(stream.clone(), max_msg_size);
    let writer_ = MsgWriter::<OutMsg, _>::new(stream, max_msg_size);
    log::info!("Socket connected");

    exec.spawn_ok(async move {
        loop {
            let msg = reader.read_message().await.unwrap();
            match msg.as_ref() {
                InMsgRef::Ai(value) => {
                    log::info!("ai: {:?}", value);
                    ai.request().await.write(value.to_native()).await;
                }
                InMsgRef::Aai(values) => {
                    log::info!("aai: {:?}", values.as_slice());
                    assert!(values.len() <= aai.max_len());
                    let mut var = aai.request().await;
                    var.clear();
                    assert_eq!(
                        var.extend_from_iter(values.iter().map(|x| x.to_native())),
                        values.len()
                    );
                    var.accept().await;
                }
                InMsgRef::Waveform(values) => {
                    log::info!("waveform: {:?}", values.as_slice());
                    assert!(values.len() <= waveform.max_len());
                    let mut var = waveform.request().await;
                    var.clear();
                    assert_eq!(
                        var.extend_from_iter(values.iter().map(|x| x.to_native())),
                        values.len()
                    );
                    var.accept().await;
                }
                InMsgRef::Bi(value) => {
                    log::info!("bi: {:?}", value);
                    bi.request().await.write(value.to_native()).await;
                }
                InMsgRef::MbbiDirect(value) => {
                    log::info!("mbbi_direct: {:?}", value);
                    mbbi_direct.request().await.write(value.to_native()).await;
                }
                InMsgRef::Stringin(string) => {
                    log::info!("stringin: {:?}", string);
                    stringin.request().await.write_from_slice(string).await;
                }
            }
        }
    });

    let writer = writer_.clone();
    exec.spawn_ok(async move {
        let mut writer = writer.clone();
        loop {
            let value = ao.acquire().await.read().await;
            log::info!("ao: {:?}", value);
            writer
                .new_message()
                .emplace(OutMsgInitAo(F64::from_native(value)))
                .unwrap()
                .write()
                .await
                .unwrap();
        }
    });

    let mut writer = writer_.clone();
    exec.spawn_ok(async move {
        loop {
            let var = aao.acquire().await;
            log::info!("aao: {:?}", var.as_slice());
            let msg = writer
                .new_message()
                .emplace(OutMsgInitAao(FromIterator(
                    var.iter().map(|x| I32::from_native(*x)),
                )))
                .unwrap();
            var.accept().await;
            msg.write().await.unwrap();
        }
    });

    let writer = writer_.clone();
    exec.spawn_ok(async move {
        let mut writer = writer.clone();
        loop {
            let value = bo.acquire().await.read().await;
            log::info!("bo: {:?}", value);
            writer
                .new_message()
                .emplace(OutMsgInitBo(U16::from_native(value)))
                .unwrap()
                .write()
                .await
                .unwrap();
        }
    });

    let writer = writer_.clone();
    exec.spawn_ok(async move {
        let mut writer = writer.clone();
        loop {
            let value = mbbo_direct.acquire().await.read().await;
            log::info!("mbbo_direct: {:?}", value);
            writer
                .new_message()
                .emplace(OutMsgInitMbboDirect(U32::from_native(value)))
                .unwrap()
                .write()
                .await
                .unwrap();
        }
    });

    let mut writer = writer_.clone();
    exec.spawn_ok(async move {
        loop {
            let var = stringout.acquire().await;
            log::info!("stringout: {:?}", var.as_slice());
            let msg = writer
                .new_message()
                .emplace(OutMsgInitStringout(FromIterator(var.iter().cloned())))
                .unwrap();
            var.accept().await;
            msg.write().await.unwrap();
        }
    });

    pending::<()>().await;
}
