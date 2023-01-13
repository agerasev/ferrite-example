mod dispatch;

use async_std::{
    future::{timeout, TimeoutError},
    main as async_main,
    net::TcpListener,
    stream::StreamExt,
    task::sleep,
};
use dispatch::Dispatcher;
use epics_ca::{
    error::Error,
    types::{EpicsEnum, EpicsString, Value},
    Context, ValueChannel,
};
use futures::{future::join_all, join, pin_mut, poll, Stream};
use rand::{
    distributions::{Alphanumeric, DistString, Standard, Uniform},
    Rng, SeedableRng,
};
use rand_distr::StandardNormal;
use rand_xoshiro::Xoroshiro128PlusPlus;
use std::{ffi::CString, pin::Pin, task::Poll, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(1);

fn cstring<S: Into<String>>(s: S) -> CString {
    let mut s = s.into();
    s.push('\0');
    CString::from_vec_with_nul(s.into()).unwrap()
}

async fn connect<V: Value + ?Sized, S: Into<String>>(ctx: &Context, name: S) -> ValueChannel<V> {
    let name = name.into();
    timeout(TIMEOUT, ctx.connect::<V>(&cstring(name.clone())))
        .await
        .unwrap_or_else(|_| panic!("Cannot connect to '{}'", name))
        .unwrap()
}

async fn next<T, S: Stream<Item = Result<T, Error>>>(
    stream: &mut Pin<&mut S>,
) -> Result<T, TimeoutError> {
    timeout(TIMEOUT, stream.next())
        .await
        .map(|x| x.unwrap().unwrap())
}

async fn box_next<T, S: Stream<Item = Result<T, Error>>>(
    stream: &mut Pin<Box<S>>,
) -> Result<T, TimeoutError> {
    timeout(TIMEOUT, stream.next())
        .await
        .map(|x| x.unwrap().unwrap())
}

#[async_main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    const SEED: u64 = 0xdeadbeef;
    const ATTEMPTS: usize = 0x1000;

    let rng = Xoroshiro128PlusPlus::seed_from_u64(SEED);
    let ctx = Context::new().unwrap();

    let server_addr = "0.0.0.0:4884";
    log::info!("Waiting for connection on {}", server_addr);
    let listener = TcpListener::bind(server_addr).await.unwrap();
    let (stream, addr) = listener.accept().await.unwrap();
    log::info!("Accepted connection from {:?}", addr);
    let dispatcher = Dispatcher::new(stream).await;

    join!(
        async {
            let mut rng = rng.clone();
            let mut input = connect::<f64, _>(&ctx, "example:ai").await;

            let mon = input.subscribe();
            pin_mut!(mon);
            assert_eq!(next(&mut mon).await.unwrap(), 0.0);

            for _ in 0..ATTEMPTS {
                let x = rng.sample(StandardNormal);
                dispatcher.ai.send(x).await.unwrap();
                assert_eq!(next(&mut mon).await.unwrap(), x);
            }

            log::info!("ai: ok");
        },
        async {
            let mut input = connect::<EpicsEnum, _>(&ctx, "example:bi").await;

            let mon = input.subscribe();
            pin_mut!(mon);
            assert_eq!(next(&mut mon).await.unwrap(), EpicsEnum(0));

            dispatcher.bi.send(1).await.unwrap();
            assert_eq!(next(&mut mon).await.unwrap(), EpicsEnum(1));

            log::info!("bi: ok");
        },
        async {
            let mut rng = rng.clone();
            let ctx = ctx.clone();
            const NBITS: usize = 32;
            let mut value: u32 = 0;
            let mut input = join_all((0..NBITS).into_iter().map(|i| {
                let ctx = ctx.clone();
                async move {
                    let name = format!("example:mbbiDirect.B{:X}", i);
                    connect::<u8, _>(&ctx, name).await
                }
            }))
            .await;

            dispatcher.mbbi_direct.send(value).await.unwrap();
            sleep(Duration::from_millis(10)).await;
            let mut monitors = input
                .iter_mut()
                .map(|chan| Box::pin(chan.subscribe_buffered()))
                .collect::<Vec<_>>();
            for mon in monitors.iter_mut() {
                assert_eq!(box_next(mon).await.unwrap(), 0);
            }
            for _ in 0..ATTEMPTS {
                let i = rng.sample(Uniform::new(0, NBITS));
                value ^= 1 << i;
                dispatcher.mbbi_direct.send(value).await.unwrap();
                let x = ((value >> i) & 1) as u8;
                assert_eq!(monitors[i].next().await.unwrap().unwrap(), x);
                for mon in monitors.iter_mut() {
                    assert!(matches!(poll!(mon.next()), Poll::Pending));
                }
            }

            log::info!("mbbiDirect: ok");
        },
        async {
            let mut rng = rng.clone();
            let mut input = connect::<EpicsString, _>(&ctx, "example:stringin").await;

            fn epics_string<S: Into<String>>(s: S) -> EpicsString {
                EpicsString::from_cstr(&cstring(s)).unwrap()
            }

            let mut prev = epics_string("");
            let mon = input.subscribe();
            pin_mut!(mon);
            assert_eq!(next(&mut mon).await.unwrap(), prev);

            for _ in 0..ATTEMPTS {
                let len = rng.sample(Uniform::new_inclusive(0, EpicsString::MAX_LEN));
                let string = epics_string(Alphanumeric.sample_string(&mut rng, len));
                if string == prev {
                    continue;
                }
                dispatcher
                    .stringin
                    .send(Vec::from(string.to_bytes()))
                    .await
                    .unwrap();
                assert_eq!(next(&mut mon).await.unwrap(), string);
                prev = string;
            }

            log::info!("stringin: ok");
        },
        async {
            let mut rng = rng.clone();
            let mut input = connect::<[i32], _>(&ctx, "example:aai").await;
            let max_len = input.element_count().unwrap();

            let mut prev = vec![];
            let mon = input.subscribe_vec();
            pin_mut!(mon);
            //assert_eq!(next(&mut mon).await.unwrap(), prev);

            for _ in 0..ATTEMPTS {
                let len = rng.sample(Uniform::new_inclusive(1, max_len));
                let vec = (&mut rng)
                    .sample_iter(Standard)
                    .take(len)
                    .collect::<Vec<_>>();
                if vec == prev {
                    continue;
                }
                dispatcher.aai.send(vec.clone()).await.unwrap();
                assert_eq!(next(&mut mon).await.unwrap(), vec);
                prev = vec;
            }

            log::info!("aai: ok");
        },
        async {
            let mut rng = rng.clone();
            let mut input = connect::<[i32], _>(&ctx, "example:waveform").await;
            let max_len = input.element_count().unwrap();

            let mut prev = vec![];
            let mon = input.subscribe_vec();
            pin_mut!(mon);
            //assert_eq!(next(&mut mon).await.unwrap(), prev);

            for _ in 0..ATTEMPTS {
                let len = rng.sample(Uniform::new_inclusive(1, max_len));
                let vec = (&mut rng)
                    .sample_iter(Standard)
                    .take(len)
                    .collect::<Vec<_>>();
                if vec == prev {
                    continue;
                }
                dispatcher.waveform.send(vec.clone()).await.unwrap();
                assert_eq!(next(&mut mon).await.unwrap(), vec);
                prev = vec;
            }

            log::info!("waveform: ok");
        },
        //
        async {
            let mut rng = rng.clone();
            let mut output = connect::<f64, _>(&ctx, "example:ao").await;

            for _ in 0..ATTEMPTS {
                let x = rng.sample(StandardNormal);
                output.put(x).unwrap().await.unwrap();
                assert_eq!(dispatcher.ao.recv().await.unwrap(), x);
            }

            log::info!("ao: ok");
        },
        async {
            let mut output = connect::<EpicsEnum, _>(&ctx, "example:bo").await;

            output.put(EpicsEnum(0)).unwrap().await.unwrap();
            assert_eq!(dispatcher.bo.recv().await.unwrap(), 0);

            output.put(EpicsEnum(1)).unwrap().await.unwrap();
            assert_eq!(dispatcher.bo.recv().await.unwrap(), 1);

            log::info!("bo: ok");
        },
        async {
            let mut rng = rng.clone();
            let ctx = ctx.clone();
            const NBITS: usize = 32;
            let mut value: u32 = 0;
            let mut output = join_all((0..NBITS).into_iter().map(|i| {
                let ctx = ctx.clone();
                async move {
                    let name = format!("example:mbboDirect.B{:X}", i);
                    connect::<u8, _>(&ctx, name).await
                }
            }))
            .await;

            for _ in 0..ATTEMPTS {
                let i = rng.sample(Uniform::new(0, NBITS));
                value ^= 1 << i;
                let x = ((value >> i) & 1) as u8;
                output[i].put(x).unwrap().await.unwrap();
                assert_eq!(dispatcher.mbbo_direct.recv().await.unwrap(), value);
            }

            log::info!("mbboDirect: ok");
        },
        async {
            let mut rng = rng.clone();
            let mut output = connect::<EpicsString, _>(&ctx, "example:stringout").await;

            fn epics_string<S: Into<String>>(s: S) -> EpicsString {
                EpicsString::from_cstr(&cstring(s)).unwrap()
            }

            let mut prev = epics_string("");
            for _ in 0..ATTEMPTS {
                let len = rng.sample(Uniform::new_inclusive(0, EpicsString::MAX_LEN));
                let string = epics_string(Alphanumeric.sample_string(&mut rng, len));
                if string == prev {
                    continue;
                }
                output.put(string).unwrap().await.unwrap();
                assert_eq!(
                    dispatcher.stringout.recv().await.unwrap(),
                    string.to_bytes()
                );
                prev = string;
            }
            log::info!("stringout: ok");
        },
        async {
            let mut rng = rng.clone();
            let mut output = connect::<[i32], _>(&ctx, "example:aao").await;
            let max_len = output.element_count().unwrap();

            let mut prev = vec![0];
            for _ in 0..ATTEMPTS {
                let len = rng.sample(Uniform::new_inclusive(1, max_len));
                let vec = (&mut rng)
                    .sample_iter(Standard)
                    .take(len)
                    .collect::<Vec<_>>();
                if vec == prev {
                    continue;
                }
                output.put_ref(&vec).unwrap().await.unwrap();
                assert_eq!(dispatcher.aao.recv().await.unwrap(), vec);
                prev = vec;
            }

            log::info!("aao: ok");
        },
    );
}
