use binance;
use binance::model::*;
use binance::websockets::*;
use colored::*;
use cryptotrader::exchanges::binance::BinanceAPI;
use cryptotrader::exchanges::ExchangeAPI;
use cryptotrader::models::group_and_average_trades_by_trade_type;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

fn get_symbols_for_aggtrades() -> Vec<String> {
    let conf = cryptotrader::config::read().unwrap();
    let keys = &conf.exchange["binance"];
    let client = BinanceAPI::connect(&keys.api_key, &keys.secret_key);

    let assets = client.balances().unwrap();

    assets
        .into_iter()
        .filter(|a| a.symbol != "BTC" && a.symbol != "USDT" && a.amount >= 10.0)
        .map(|a| a.symbol.to_lowercase())
        .collect()
}

fn main() {
    let (tx, rx) = mpsc::channel();

    fn attach_ws(asset: String, tx: mpsc::Sender<(String, f64)>) {
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

            let agg_trade: String = format!("{}btc@aggTrade", asset);
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

    let mut prices: HashMap<String, Price> = HashMap::new();
    let assets = get_symbols_for_aggtrades();
    for asset in assets {
        let btc_pair = format!("{}BTC", asset.to_uppercase());

        let conf = cryptotrader::config::read().unwrap();
        let keys = &conf.exchange["binance"];
        let client = BinanceAPI::connect(&keys.api_key, &keys.secret_key);

        let pairs = client.all_pairs().expect("pairs to unwrap");

        println!("attempting to fetch trades for {}...", asset);

        if let Ok(trades) = client.trades_for_symbol(&asset, pairs) {
            if let Some(trade) = group_and_average_trades_by_trade_type(trades).last() {
                prices.insert(
                    btc_pair,
                    Price {
                        entry_price: trade.price,
                        current_price: trade.price,
                        position_size: trade.qty,
                    },
                );
                attach_ws(asset, tx.clone());
            }
        }
    }

    println!("listening...");

    loop {
        if let Ok(r) = rx.recv() {
            let (symbol, price) = r;

            if let Some(new_price) = prices.get_mut(&symbol) {
                new_price.current_price = price;
            } else {
                println!("ERROR: COULD NOT WRITE {} {:?}", symbol, prices);
            }

            fn display_ticker(prices: HashMap<String, Price>) {
                let p = prices
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

            display_ticker(prices.clone());
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
        println!("{}", String::from_utf8_lossy(&output.stdout));
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
