#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(async_closure)]

extern crate serde;
extern crate baseplug;
extern crate buttplug;

//use core::panic::PanicInfo;
use futures::StreamExt;
use tokio::{self, sync::mpsc, runtime::Runtime};
use log::LevelFilter;
use serde::{Serialize, Deserialize};
use log4rs::{
    append::file::FileAppender,
    encode::pattern::PatternEncoder,
    config::{Appender, Config, Root}
};
use baseplug::{
    ProcessContext,
    Plugin,
};
use buttplug::{
    client::{ButtplugClient, ButtplugClientEvent},
    connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport},
    core::messages::serializer::ButtplugClientJSONSerializer,
  };

/*struct ButtplugBuffer{
    l: Vec<f32>,
    r: Vec<f32>,
}
impl ButtplugBuffer{
    fn new() -> ButtplugBuffer{
        ButtplugBuffer{
            l: Vec::new(),
            r: Vec::new(),
        }
    }
}
unsafe impl Send for ButtplugBuffer {}*/

/*#[panic_handler]
fn on_panic(info: &PanicInfo) -> ! {

    // logs "panicked at '$reason', src/main.rs:27:4" to the host stderr
    log::info!("{}", info);

    loop {}
}*/

async fn buttplug_thread(mut receiver: mpsc::Receiver<f32>){
    
    let connector = ButtplugRemoteClientConnector::<
            ButtplugWebsocketClientTransport,
            ButtplugClientJSONSerializer,
        >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
            "ws://127.0.0.1:12345",
        ));
    let client = ButtplugClient::new("Buttplug VST Client");
    log::info!("Starting Buttplug connection thread...");

    if let Err(e) = client.connect(connector).await { // PANICS?
        log::info!("Error connecting to Buttplug server: {}", e);
        return;
    }
    log::info!("Connected: {}", client.connected());

    let mut event_stream = client.event_stream();
    loop{
        tokio::select!{
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(event) => match event {
                        ButtplugClientEvent::ServerDisconnect => {
                            log::info!("Server disconnected");
                            break;
                        },
                        _ => {}
                    }
                    None => {
                        log::info!("Server disconnected ungracefully");
                        break;
                    }
                }
                    
            }

            maybe_msg = receiver.recv() => {
                match maybe_msg {
                    Some(msg) => {

                    }
                    None => {
                        log::info!("Sender dropped, killing buttplug thread");
                        break;
                    }
                }
            }
        }
    }
}


baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct ButtplugModel {
        #[model(min = -90.0, max = 3.0)]
        #[parameter(name = "gain", unit = "Decibels",
            gradient = "Power(0.15)")]
        gain: f32,
    }
}

impl ButtplugModel {
    const SEND_RATE: f32 = 20.0; // Per second
}

impl Default for ButtplugModel {

    fn default() -> Self {
        Self {
            // "gain" is converted from dB to coefficient in the parameter handling code,
            // so in the model here it's a coeff.
            // -0dB == 1.0
            gain: 0.0,
        }
    }
}

struct ButtplugMonitor {
    bpio_sender: mpsc::Sender<f32>,
}

impl Plugin for ButtplugMonitor {
    const NAME: &'static str = "Buttplug Monitor";
    const PRODUCT: &'static str = "Buttplug Monitor";
    const VENDOR: &'static str = "Fractalysoft";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = ButtplugModel;

    #[inline]
    fn new(_sample_rate: f32, _model: &ButtplugModel) -> Self {
        //Enable logging
        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
            .build("E:/Users/facade/Documents/VSTs/logs/buttplug_monitor.log")
            .unwrap();
        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(Root::builder()
                    .appender("logfile")
                    .build(LevelFilter::Info)).unwrap();
        let _ = log4rs::init_config(config);
        log::info!("Logging enabled"); // For some reason this runs 4 times


        let buffer_size = (2.0*_sample_rate/ButtplugModel::SEND_RATE).ceil() as usize;
        let (sender, receiver): (mpsc::Sender<f32>, mpsc::Receiver<f32>) = mpsc::channel( buffer_size );

        let tkrt = Runtime::new().unwrap();
        tkrt.spawn(async move {
            buttplug_thread(receiver).await;
        });

        log::info!("Thread spawned successfully");

        ButtplugMonitor {
            bpio_sender: sender,
        }
    }

    #[inline]
    fn process(&mut self, _model: &ButtplugModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;

        for i in 0..ctx.nframes {
            output[0][i] = input[0][i];
            output[1][i] = input[1][i];

            // Will not block
            let _ = self.bpio_sender.try_send(input[0][i]);
            let _ = self.bpio_sender.try_send(input[1][i]);
        }
    }
}

baseplug::vst2!(ButtplugMonitor, b"FRbm");