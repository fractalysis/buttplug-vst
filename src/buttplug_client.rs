//#![allow(unused_variables)]
//#![allow(unused_imports)]

extern crate buttplug;

use tokio::{
    self,
    sync::mpsc,
    runtime::Runtime
};
use futures::StreamExt;
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    encode::pattern::PatternEncoder,
    config::{Appender, Config, Root}
};
use buttplug::{
    client::{ButtplugClient, ButtplugClientEvent},
    connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport},
    core::messages::serializer::ButtplugClientJSONSerializer,
};

//Debug
use url::Url;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures::SinkExt;

use std::error::Error;


pub fn start_buttplug_thread(send_rate: f32, sample_rate: f32) -> Result<(Runtime, mpsc::Sender<f32>), ()>{
    //Enable logging
    let logfile = match FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("E:/Users/facade/Documents/VSTs/logs/buttplug_monitor.log"){
        Ok(logfile) => logfile,
        Err(e) => {
            log::info!("{}", e);
            return Err(());
        }
    };
    let config = match Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder()
            .appender("logfile")
            .build(LevelFilter::Info)) {
        Ok(c) => c,
        Err(e) => {
            log::info!("{}", e);
            return Err(());
        }
    };
    let _ = log4rs::init_config(config);
    log::info!("Logging enabled"); // For some reason this runs 4 times with baseplug


    let buffer_size = (2.0*sample_rate/send_rate).ceil() as usize; //Should get sample rate instead
    let (sender, receiver): (mpsc::Sender<f32>, mpsc::Receiver<f32>) = mpsc::channel( buffer_size );

    let tkrt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::info!("{}", e);
            return Err(());
        }
    };

    tkrt.spawn(async move {
        buttplug_thread(receiver).await;
        //websocket_test_thread().await;
    });
    log::info!("Buttplug thread spawned.");

    Ok((tkrt, sender))
}

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
        Err(e) => log::info!("{}", e)
    }
}

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