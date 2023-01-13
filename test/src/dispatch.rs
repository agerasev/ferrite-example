use async_std::{
    channel::{unbounded, Receiver, Sender},
    net::TcpStream,
    task::spawn,
};
use flatty::{
    portable::{le::*, NativeCast},
    vec::FromIterator,
};
use flatty_io::{AsyncReader as MsgReader, AsyncWriter as MsgWriter};
use futures::{select, FutureExt};
use protocol::*;

pub struct Dispatcher {
    pub ai: Sender<f64>,
    pub aai: Sender<Vec<i32>>,
    pub waveform: Sender<Vec<i32>>,
    pub bi: Sender<u16>,
    pub mbbi_direct: Sender<u32>,
    pub stringin: Sender<Vec<u8>>,

    pub ao: Receiver<f64>,
    pub aao: Receiver<Vec<i32>>,
    pub bo: Receiver<u16>,
    pub mbbo_direct: Receiver<u32>,
    pub stringout: Receiver<Vec<u8>>,
}

impl Dispatcher {
    pub async fn new(stream: TcpStream) -> Self {
        let mut reader = MsgReader::<OutMsg, _>::new(stream.clone(), MAX_MSG_SIZE);
        let mut writer = MsgWriter::<InMsg, _>::new(stream, MAX_MSG_SIZE);

        let (ai_s, ai_r) = unbounded();
        let (aai_s, aai_r) = unbounded::<Vec<_>>();
        let (waveform_s, waveform_r) = unbounded::<Vec<_>>();
        let (bi_s, bi_r) = unbounded();
        let (mbbi_direct_s, mbbi_direct_r) = unbounded();
        let (stringin_s, stringin_r) = unbounded::<Vec<_>>();

        let (ao_s, ao_r) = unbounded();
        let (aao_s, aao_r) = unbounded();
        let (bo_s, bo_r) = unbounded();
        let (mbbo_direct_s, mbbo_direct_r) = unbounded();
        let (stringout_s, stringout_r) = unbounded();

        spawn(async move {
            loop {
                let msg = reader.read_message().await.unwrap();
                match msg.as_ref() {
                    OutMsgRef::Ao(value) => ao_s.send(value.to_native()).await.unwrap(),
                    OutMsgRef::Aao(values) => aao_s
                        .send(values.iter().map(|x| x.to_native()).collect())
                        .await
                        .unwrap(),
                    OutMsgRef::Bo(value) => bo_s.send(value.to_native()).await.unwrap(),
                    OutMsgRef::MbboDirect(value) => {
                        mbbo_direct_s.send(value.to_native()).await.unwrap()
                    }
                    OutMsgRef::Stringout(values) => stringout_s
                        .send(values.iter().cloned().collect())
                        .await
                        .unwrap(),
                }
            }
        });

        spawn(async move {
            loop {
                let uninit_msg = writer.new_message();
                let msg = select! {
                    value = ai_r.recv().map(|r| r.unwrap()) => {
                        uninit_msg.emplace(InMsgInitAi(F64::from_native(value))).unwrap()
                    },
                    values = aai_r.recv().map(|r| r.unwrap()) => {
                        uninit_msg.emplace(InMsgInitAai(FromIterator(
                            values.into_iter().map(I32::from_native),
                        )))
                        .unwrap()
                    },
                    values = waveform_r.recv().map(|r| r.unwrap()) => {
                        uninit_msg.emplace(InMsgInitWaveform(FromIterator(
                            values.into_iter().map(I32::from_native),
                        )))
                        .unwrap()
                    },
                    value = bi_r.recv().map(|r| r.unwrap()) => {
                        uninit_msg.emplace(InMsgInitBi(U16::from_native(value))).unwrap()
                    },
                    value = mbbi_direct_r.recv().map(|r| r.unwrap()) => {
                        uninit_msg.emplace(InMsgInitMbbiDirect(U32::from_native(value))).unwrap()
                    },
                    string = stringin_r.recv().map(|r| r.unwrap()) => {
                        uninit_msg.emplace(InMsgInitStringin(FromIterator(string.into_iter())))
                            .unwrap()
                    },
                };
                msg.write().await.unwrap();
            }
        });

        Dispatcher {
            ai: ai_s,
            aai: aai_s,
            waveform: waveform_s,
            bi: bi_s,
            mbbi_direct: mbbi_direct_s,
            stringin: stringin_s,

            ao: ao_r,
            aao: aao_r,
            bo: bo_r,
            mbbo_direct: mbbo_direct_r,
            stringout: stringout_r,
        }
    }
}
