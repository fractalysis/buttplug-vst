//#![allow(unused_variables)]
//#![allow(unused_imports)]

extern crate buttplug;

use buttplug::{
    client::{
        ButtplugClient, ButtplugClientDevice, ButtplugClientDeviceMessageType, ButtplugClientEvent,
        VibrateCommand,
    },
    connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport},
    core::messages::serializer::ButtplugClientJSONSerializer,
};
use futures::StreamExt;
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};
use std::sync::Arc;
use tokio::{self, runtime::Runtime, sync::mpsc, sync::mpsc::error::TryRecvError, time};

//Debug
use futures::SinkExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

pub fn start_buttplug_thread(send_rate: f32) -> Result<(Runtime, mpsc::Sender<f32>), ()> {
    //Enable logging
    let logfile = match FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("E:/Users/facade/Documents/VSTs/logs/buttplug_monitor.log")
    {
        Ok(logfile) => logfile,
        Err(e) => {
            log::info!("{}", e);
            return Err(());
        }
    };
    let config = match Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
    {
        Ok(c) => c,
        Err(e) => {
            log::info!("{}", e);
            return Err(());
        }
    };
    let _ = log4rs::init_config(config);
    log::info!("Logging enabled"); // For some reason this runs 4 times with baseplug

    let buffer_size = (100.0 / send_rate) as usize; // Should be big enough
    let (sender, receiver): (mpsc::Sender<f32>, mpsc::Receiver<f32>) = mpsc::channel(buffer_size);

    let tkrt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::info!("{}", e);
            return Err(());
        }
    };

    tkrt.spawn(async move {
        buttplug_thread(receiver, send_rate).await;
        //websocket_test_thread().await;
    });
    log::info!("Buttplug thread spawned.");

    Ok((tkrt, sender))
}

// For debugging the thread and whether it can network
#[allow(dead_code)]
async fn websocket_test_thread() {
    let url = Url::parse("ws://localhost:12345/").unwrap();
    let (ws_stream, _) = match connect_async(url).await {
        Ok(ws_stream) => ws_stream,
        Err(e) => {
            log::info!("{}", e);
            return;
        }
    };

    let (mut write, _) = ws_stream.split();

    match write.send(Message::Text("Hello, world!".to_string())).await {
        Ok(_) => log::info!("Sent message"),
        Err(e) => log::info!("{}", e),
    }
}

async fn buttplug_thread(mut receiver: mpsc::Receiver<f32>, send_rate: f32) {
    let connector = ButtplugRemoteClientConnector::<
        ButtplugWebsocketClientTransport,
        ButtplugClientJSONSerializer,
    >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
        "ws://127.0.0.1:12345",
    ));
    let client = ButtplugClient::new("Buttplug VST Client");
    log::info!("Starting Buttplug connection thread...");

    if let Err(e) = client.connect(connector).await {
        // PANICS?
        log::info!("Error connecting to Buttplug server: {}", e);
        return;
    }
    log::info!("Connected: {}", client.connected());

    if let Err(err) = client.start_scanning().await {
        log::info!("Client errored when starting device scan: {}", err);
        return;
    }

    //Store the latest device here, assume it vibrates, None if there are no devices connected
    let mut device: Option<Arc<ButtplugClientDevice>> = None;
    for d in client.devices().iter() {
        log::info!("Device found: {}", d.name);
        device = Some(d.clone());
    }

    //Begin processing audio on an interval
    let mut audio_interval = time::interval(std::time::Duration::from_millis(
        (1000.0 / send_rate).round() as u64,
    ));

    let mut event_stream = client.event_stream();
    loop {
        tokio::select! {

            // Buttplug server message events
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(event) => match event {
                        ButtplugClientEvent::ServerDisconnect => {
                            log::info!("Server disconnected");
                            break;
                        },

                        ButtplugClientEvent::DeviceAdded(d) => {
                            if d.allowed_messages
                                .contains_key(&ButtplugClientDeviceMessageType::VibrateCmd)
                            {
                                log::info!("Vibrating device added: {}", d.name);
                                device = Some(d.clone());
                            }
                            else{
                                log::info!("Non-vibrating device added, ignoring: {}", d.name);
                            }
                        },

                        ButtplugClientEvent::DeviceRemoved(d) => {
                            log::info!("Device removed: {}", d.name);
                            if let Some(dev) = &device {
                                if d == *dev {
                                    device = None;
                                }
                            }
                        },

                        _ => {
                            log::info!("Intiface event: {:?}", event);
                        }
                    }
                    None => {
                        log::info!("Server disconnected ungracefully");
                        break;
                    }
                }

            }

            // A timer event that receives audio
            _ = audio_interval.tick() => {
                match receiver.try_recv() {
                    Ok(msg) => {
                        if let Some(dev) = &device {
                            if let Err(e) = dev.vibrate(VibrateCommand::Speed(f64::from(msg))).await {
                                log::info!("Error sending vibrate command: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        if e == TryRecvError::Empty {
                            //log::info!("Sound queue was empty, continuing");
                        }
                        else if e == TryRecvError::Disconnected {
                            log::info!("Sender disconnected, exiting");
                            break;
                        }
                    }
                }
            }

        }
    }
}
