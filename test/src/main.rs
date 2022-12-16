use async_std::{
    future::{timeout, TimeoutError},
    main as async_main,
    stream::StreamExt,
};
use epics_ca::{
    error::Error,
    types::{EpicsEnum, EpicsString, Value},
    Context, ValueChannel,
};
use futures::{future::join_all, join, pin_mut, Stream};
use rand::{
    distributions::{Alphanumeric, DistString, Standard, Uniform},
    Rng, SeedableRng,
};
use rand_distr::StandardNormal;
use rand_xoshiro::Xoroshiro128PlusPlus;
use std::{ffi::CString, pin::Pin, time::Duration};

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
    const SEED: u64 = 0xdeadbeef;
    const ATTEMPTS: usize = 0x1000;

    let rng = Xoroshiro128PlusPlus::seed_from_u64(SEED);
    let ctx = Context::new().unwrap();

    join!(
        async {
            let mut rng = rng.clone();
            let mut input = connect::<f64, _>(&ctx, "example:ai").await;
            let mut output = connect::<f64, _>(&ctx, "example:ao").await;

            output.put(0.0).unwrap().await.unwrap();
            let mon = input.subscribe();
            pin_mut!(mon);
            assert_eq!(next(&mut mon).await.unwrap(), 0.0);

            for _ in 0..ATTEMPTS {
                let x = rng.sample(StandardNormal);
                output.put(x).unwrap().await.unwrap();
                assert_eq!(next(&mut mon).await.unwrap(), x);
            }

            println!("ao -> ai: ok");
        },
        async {
            let mut input = connect::<EpicsEnum, _>(&ctx, "example:bi").await;
            let mut output = connect::<EpicsEnum, _>(&ctx, "example:bo").await;

            output.put(EpicsEnum(0)).unwrap().await.unwrap();
            let mon = input.subscribe();
            pin_mut!(mon);
            assert_eq!(next(&mut mon).await.unwrap(), EpicsEnum(0));

            output.put(EpicsEnum(1)).unwrap().await.unwrap();
            assert_eq!(next(&mut mon).await.unwrap(), EpicsEnum(1));

            println!("bo -> bi: ok");
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
                    let mut chan = connect::<u8, _>(&ctx, name).await;
                    chan.put(0).unwrap().await.unwrap();
                    chan
                }
            }))
            .await;
            let mut input = join_all((0..NBITS).into_iter().map(|i| {
                let ctx = ctx.clone();
                async move {
                    let name = format!("example:mbbiDirect.B{:X}", i);
                    connect::<u8, _>(&ctx, name).await
                }
            }))
            .await;

            let mut monitors = input
                .iter_mut()
                .map(|chan| Box::pin(chan.subscribe()))
                .collect::<Vec<_>>();
            for mon in monitors.iter_mut() {
                assert_eq!(box_next(mon).await.unwrap(), 0);
            }
            for _ in 0..ATTEMPTS {
                let i = rng.sample(Uniform::new(0, NBITS));
                value ^= 1 << i;
                let x = ((value >> i) & 1) as u8;
                output[i].put(x).unwrap().await.unwrap();
                assert_eq!(monitors[i].next().await.unwrap().unwrap(), x);
            }

            println!("mbboDirect -> mbbiDirect: ok");
        },
        async {
            let mut rng = rng.clone();
            let mut input = connect::<EpicsString, _>(&ctx, "example:stringin").await;
            let mut output = connect::<EpicsString, _>(&ctx, "example:stringout").await;

            fn epics_string<S: Into<String>>(s: S) -> EpicsString {
                EpicsString::from_cstr(&cstring(s)).unwrap()
            }

            let mut prev = epics_string("");
            output.put(prev).unwrap().await.unwrap();
            let mon = input.subscribe();
            pin_mut!(mon);
            assert_eq!(next(&mut mon).await.unwrap(), prev);

            for _ in 0..ATTEMPTS {
                let len = rng.sample(Uniform::new_inclusive(0, EpicsString::MAX_LEN));
                let string = epics_string(Alphanumeric.sample_string(&mut rng, len));
                if string == prev {
                    continue;
                }
                output.put(string).unwrap().await.unwrap();
                assert_eq!(next(&mut mon).await.unwrap(), string);
                prev = string;
            }

            let string = epics_string("@".repeat(EpicsString::MAX_LEN));
            assert_ne!(string, prev);
            output.put(string).unwrap().await.unwrap();
            assert_eq!(next(&mut mon).await.unwrap(), string);

            println!("stringout -> stringin: ok");
        },
        async {
            let mut rng = rng.clone();
            let mut output = connect::<[i32], _>(&ctx, "example:aao").await;
            let mut input = connect::<[i32], _>(&ctx, "example:aai").await;
            let mut waveform = connect::<[i32], _>(&ctx, "example:waveform").await;

            let mut prev = vec![0];
            output.put_ref(&prev).unwrap().await.unwrap();
            let mon = input.subscribe_vec();
            let mon_wf = waveform.subscribe_vec();
            pin_mut!(mon, mon_wf);
            assert_eq!(next(&mut mon).await.unwrap(), prev);
            assert_eq!(next(&mut mon_wf).await.unwrap(), prev);

            let max_len = output.element_count().unwrap();
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
                assert_eq!(next(&mut mon).await.unwrap(), vec);
                assert_eq!(next(&mut mon_wf).await.unwrap(), vec);
                prev = vec;
            }

            let vec = [-1].repeat(max_len);
            assert_ne!(vec, prev);
            output.put_ref(&vec).unwrap().await.unwrap();
            assert_eq!(next(&mut mon).await.unwrap(), vec);
            assert_eq!(next(&mut mon_wf).await.unwrap(), vec);

            println!("aao -> (aai, waveform): ok");
        },
    );
}
