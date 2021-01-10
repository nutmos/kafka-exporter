use clap::{App, Arg};
use futures::StreamExt;
use log::{info, debug, error};
use std::time::Duration;

use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
//use rdkafka::message::{Headers, Message};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::statistics::Statistics;
use rdkafka::util::get_rdkafka_version;

//use crate::example_utils::setup_logger;

use std::net::SocketAddr;
use std::mem::drop;
use prometheus_exporter_base::{render_prometheus, MetricType, PrometheusMetric};
use futures::future::{ok};
//use futures::join;
use futures::{future};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref KAFKA_STAT: Mutex<Option<Statistics>> = Mutex::new(None);
    static ref KAFKA_BROKERS: Mutex<Option<String>> = Mutex::new(None);
    static ref KAFKA_CONSUMER_GROUP_ID: Mutex<Option<String>> = Mutex::new(None);
}

//mod example_utils;

// A context can be used to change the behavior of producers and consumers by adding callbacks
// that will be executed by librdkafka.
// This particular context sets up custom callbacks to log rebalancing events.
struct CustomContext;

impl ClientContext for CustomContext {
    fn stats(&self, statistics: Statistics) {
        let mut guard = KAFKA_STAT.lock().unwrap();
        debug!("Get Stat Completed {:?}", statistics);
        *guard = Some(statistics);
        drop(guard);
    }
}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        info!("Committing offsets: {:?}", result);
    }
}

// A type alias with your custom consumer can be created for convenience.
type LoggingConsumer = StreamConsumer<CustomContext>;

async fn consume_and_print(brokers: &str, group_id: &str, topics: &[&str]) {
    let context = CustomContext;

    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", group_id)
        .set("bootstrap.servers", brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");

    consumer
        .subscribe(&topics.to_vec())
        .expect("Can't subscribe to specified topics");
    info!("In function test1");

    // consumer.start() returns a stream. The stream can be used ot chain together expensive steps,
    // such as complex computations on a thread pool or asynchronous IO.
    let mut message_stream = consumer.start();

    info!("In function test2");

    while let Some(message) = message_stream.next().await {
    }
    

    //while let Some(message) = message_stream.next().await {
    //    match message {
    //        Err(e) => warn!("Kafka error: {}", e),
    //        Ok(m) => {
    //            let payload = match m.payload_view::<str>() {
    //                None => "",
    //                Some(Ok(s)) => s,
    //                Some(Err(e)) => {
    //                    warn!("Error while deserializing message payload: {:?}", e);
    //                    ""
    //                }
    //            };
    //            info!("key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
    //                  m.key(), payload, m.topic(), m.partition(), m.offset(), m.timestamp());
    //            if let Some(headers) = m.headers() {
    //                for i in 0..headers.count() {
    //                    let header = headers.get(i).unwrap();
    //                    info!("  Header {:#?}: {:?}", header.0, header.1);
    //                }
    //            }
    //            consumer.commit_message(&m, CommitMode::Async).unwrap();
    //        }
    //    };
    //}
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = App::new("consumer example")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Simple command line consumer")
        .arg(
            Arg::with_name("brokers")
                .short("b")
                .long("brokers")
                .help("Broker list in kafka format")
                .takes_value(true)
                .default_value("localhost:9092"),
        )
        .arg(
            Arg::with_name("group-id")
                .short("g")
                .long("group-id")
                .help("Consumer group id")
                .takes_value(true)
                .default_value("example_consumer_group_id"),
        )
        .arg(
            Arg::with_name("log-conf")
                .long("log-conf")
                .help("Configure the logging format (example: 'rdkafka=trace')")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("topics")
                .short("t")
                .long("topics")
                .help("Topic list")
                .takes_value(true)
                .multiple(true)
                .default_value("test-exporter")
                //.required(true),
        )
        .get_matches();

    //setup_logger(true, matches.value_of("log-conf"));

    let (version_n, version_s) = get_rdkafka_version();
    info!("rd_kafka_version: 0x{:08x}, {}", version_n, version_s);

    let topics = matches.values_of("topics").unwrap().collect::<Vec<&str>>();
    let brokers = matches.value_of("brokers").unwrap();
    let group_id = matches.value_of("group-id").unwrap();

    let mut brokers_mutex = KAFKA_BROKERS.lock().unwrap();
    *brokers_mutex = Some(String::from(brokers));
    drop(brokers_mutex);
    let mut group_id_mutex = KAFKA_CONSUMER_GROUP_ID.lock().unwrap();
    *group_id_mutex = Some(String::from(group_id));
    drop(group_id_mutex);

    let kafka_async = consume_and_print(brokers, group_id, &topics);

    let addr = SocketAddr::from(([0, 0, 0, 0], 32221));
    info!("Kafka Prometheus Exporter");
    info!("Listen on {}", addr);

    let prom_async = render_prometheus(addr, {}, |_request, _options| {
        info!("Test");
        let mut s = "".to_string();
        let mut guard = KAFKA_STAT.lock().unwrap();
        match &*guard {
            None => info!("Could not get Kafka stat"),
            Some(stat) => {
                info!("Get stat for broker {} completed: {} brokers", stat.name, stat.brokers.keys().len());
                for broker in stat.brokers.iter() {
                    let (key, val) = broker;
                    //info!("Get stat for broker {} {}", key, val.name);
                    if key != "GroupCoordinator" {
                        let mut labels = Vec::new();
                        labels.push(("kafka_broker", &key[..]));
                        let pc = PrometheusMetric::new("kafka_broker_partitions_count", MetricType::Gauge, "Kafka Broker Partitions Count");
                        s += &pc.render_header();
                        s.push_str(&pc.render_sample(Some(&labels[..]), val.toppars.keys().len(), None));
                    } else {
                        let mut labels = Vec::new();
                        labels.push(("kafka_group_coordinator_broker", &val.nodename[..]));
                        let pc = PrometheusMetric::new("kafka_group_coordinator_broker", MetricType::Gauge, "Kafka Coordinator Broker");
                        s += &pc.render_header();
                        s.push_str(&pc.render_sample(Some(&labels[..]), 1, None));
                    }
                }
                info!("Get stat for broker {} completed: {} topics", stat.name, stat.topics.keys().len());
                for topic in stat.topics.iter() {
                    let (key, val) = topic;
                    info!("Get stat for topic {}", val.topic);
                }
                info!("Get stat completed");
                let context = CustomContext;
                let mut group_id = "";
                let mut group_id_guard = KAFKA_CONSUMER_GROUP_ID.lock().unwrap();
                match &*group_id_guard {
                    None => error!("Kafka consumer group id not set"),
                    Some(g) => {
                        group_id = g;
                    }
                }
                let mut brokers = "";
                let mut brokers_guard = KAFKA_BROKERS.lock().unwrap();
                match &*brokers_guard {
                    None => error!("Kafka brokers not set"),
                    Some(b) => {
                        brokers = b;
                    }
                }
                
                info!("Start consumer");
                let consumer: LoggingConsumer = ClientConfig::new()
                    .set("group.id", group_id)
                    .set("bootstrap.servers", brokers)
                    .set("enable.partition.eof", "false")
                    .set("session.timeout.ms", "6000")
                    .set("enable.auto.commit", "true")
                    .set("statistics.interval.ms", "30000")
                    //.set("auto.offset.reset", "smallest")
                    .set_log_level(RDKafkaLogLevel::Debug)
                    .create_with_context(context)
                    .expect("Consumer creation failed");
                drop(group_id_guard);
                drop(brokers_guard);
                let metadata = consumer.fetch_metadata(None, Duration::from_millis(60000));
                match metadata {
                    Err(e) => info!("Fetch Metadata Error"),
                    Ok(m) => {
                        let topics = m.topics();
                        let pc = PrometheusMetric::new("kafka_topics_count", MetricType::Gauge, "Kafka Topics Count");
                        s += &pc.render_header();
                        s.push_str(&pc.render_sample(None, topics.len(), None));
                        for topic in topics.iter() {
                            let mut labels = Vec::new();
                            labels.push(("kafka_topic_name", topic.name()));
                            let pc = PrometheusMetric::new("kafka_topic_partitions_count", MetricType::Gauge, "Kafka Topic Partitions Count");
                            s += &pc.render_header();
                            s.push_str(&pc.render_sample(Some(&labels[..]), topic.partitions().len(), None));
                        }
                    },
                }
            },
        }
        ok(s)
    });
    future::join(consume_and_print(brokers, group_id, &topics), prom_async).await;
}
