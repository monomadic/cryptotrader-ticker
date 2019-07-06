use binance;
use binance::model::*;
use binance::websockets::*;
use colored::*;
use cryptotrader::exchanges::binance::BinanceAPI;
use cryptotrader::exchanges::ExchangeAPI;
use cryptotrader::models::PriceUtils;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

mod config;

use config::*;

fn split_market(mapentry: (String, PairListing)) -> Pair {
    let (market, pairlist) = mapentry;
    let mut splitmarket = market.split('_');
    let symbol = splitmarket.next().unwrap();
    let base = splitmarket.next().unwrap();

    match pairlist {
        PairListing::Simple(entry) => Pair {
            base: base.to_string(),
            symbol: symbol.to_string(),
            entry: entry,
        },
        PairListing::Detailed(pair) => pair,
    }
}

fn get_symbols_for_aggtrades() -> Vec<config::Pair> {
    let conf = config::read().unwrap();
    // println!("config loaded: {:?}", conf.clone());
    let binance_assets: Vec<config::Pair> = conf
        .binance
        .unwrap()
        .into_iter()
        .map(split_market)
        .collect();

    binance_assets
}

fn main() {
    let (tx, rx) = mpsc::channel();

    fn attach_ws(pair: String, tx: mpsc::Sender<(String, f64)>) {
        thread::spawn(move || {
            // LAUNCH WEBSOCKET

            #[derive(Clone)]
            struct WebSocketHandler {
                tx: mpsc::Sender<(String, f64)>,
            };

            impl MarketEventHandler for WebSocketHandler {
                fn aggregated_trades_handler(&self, event: &TradesEvent) {
                    let price = event.price.parse::<f64>().unwrap();
                    self.tx.send((event.symbol.clone(), price)).unwrap();
                }
                fn depth_orderbook_handler(&self, model: &DepthOrderBookEvent) {
                    println!("- Depth Order Book: {:?}", model);
                }
                fn partial_orderbook_handler(&self, model: &OrderBook) {
                    println!("- Partial Order Book: {:?}", model);
                }
            }

            // RUN WS

            let agg_trade: String = format!("{}@aggTrade", pair.to_lowercase());
            let mut web_socket: WebSockets = WebSockets::new();

            println!("attaching websocket handler to {}", agg_trade);

            web_socket.add_market_handler(WebSocketHandler { tx: tx.clone() });
            web_socket.connect(&agg_trade).unwrap(); // check error
            web_socket.event_loop();

            // END WS
        });
    };

    #[derive(Clone, Debug)]
    struct Price {
        entry_price: f64,
        current_price: f64,
        position_size: f64,
    }

    let mut asset_map: HashMap<String, Price> = HashMap::new();

    let assets = get_symbols_for_aggtrades();

    // let conf = cryptotrader::config::read().unwrap();
    // let keys = &conf.exchange["binance"];
    // let client = BinanceAPI::new(&keys.api_key, &keys.secret_key);
    let client = BinanceAPI::new();
    let prices = client.all_prices().expect("pairs to unwrap");
    // let _btc_price = prices
    //     .price_for(client.btc_usd_pair())
    //     .expect("btc price not found");

    // insert btc as an update pair
    // asset_map.insert(
    //     "BTCUSDT".to_string(),
    //     Price {
    //         entry_price: btc_price,
    //         current_price: btc_price,
    //         position_size: 0.0,
    //     },
    // );
    // attach_ws("btcusdt".to_string(), tx.clone());

    for pair in assets {
        println!("attempting to fetch trades for {}...", pair.base);
        attach_ws(format!("{}{}", pair.symbol, pair.base), tx.clone());
        let current_price = prices
            .price_of(&pair.symbol.to_uppercase(), &pair.base)
            .expect(&format!("price to exist: {} {}", pair.symbol, pair.base));
        asset_map.insert(
            format!("{}{}", pair.symbol, pair.base).to_uppercase(),
            Price {
                entry_price: pair.entry,
                current_price: current_price,
                position_size: 0.0,
            },
        );
    }

    println!("listening...");

    loop {
        if let Ok(r) = rx.recv() {
            let (symbol, price) = r;

            if let Some(new_price) = asset_map.get_mut(&symbol) {
                new_price.current_price = price;
            } else {
                println!(
                    "ERROR: COULD NOT WRITE {} {:?} {:?}",
                    symbol, prices, asset_map
                );
            }

            fn display_ticker(prices: HashMap<String, Price>) {
                let p = prices
                    .clone()
                    .into_iter()
                    .map(|(symbol, price)| {
                        let price_percent = price_percent(price.entry_price, price.current_price);
                        format!(
                            "{} {}",
                            symbol.to_uppercase().yellow(),
                            // price.position_size,
                            positive_negative(price_percent, format!("{:.2}%", price_percent)),
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(" :: ");

                cls();
                println!("{}", p);
            }

            display_ticker(asset_map.clone());
        }
    }
}

/// Expresses the difference as a percentage between two floats.
///
/// ```rust
/// use cryptotrader::presenters::price_percent;
/// assert_eq!(price_percent(5.0, 10.0), 100.0);
/// assert_eq!(price_percent(100.0, 50.0), -50.0);
/// ```
pub fn price_percent(entry_price: f64, exit_price: f64) -> f64 {
    if entry_price < exit_price {
        (100. / entry_price * exit_price) - 100.
    } else {
        -(100. + -100. / entry_price * exit_price)
    }
}

pub fn cls() {
    use std::process::Command;

    if let Ok(output) = Command::new("clear").output() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
}

pub fn positive_negative(number: f64, string: String) -> ColoredString {
    if number > 0.01 {
        string.green()
    } else if number < -0.01 {
        string.red()
    } else {
        string.normal()
    }
}
