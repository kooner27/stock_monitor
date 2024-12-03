use yahoo_finance_api as yahoo;
use plotly::candlestick::Candlestick;
use plotly::common::{Marker, Mode, Title};
use plotly::layout::{Axis, Layout};
use plotly::{Plot, Scatter};
use chrono::{DateTime, TimeZone, Utc};
use tokio;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    let mut ticker = String::new();
    print!("Enter the stock ticker (e.g., AAPL): ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut ticker).unwrap();
    let ticker = ticker.trim().to_uppercase();

    let provider = yahoo::YahooConnector::new().unwrap();

    match fetch_stock_data(&provider, &ticker, "1d", "6mo").await {
        Ok(quotes) => {
            println!("Fetched {} quotes for {}", quotes.len(), ticker);
            let output_file = format!("{}_stock_chart.html", ticker);
            if let Err(e) = generate_candlestick_with_volatility_html(&ticker, &quotes, &output_file) {
                eprintln!("Failed to generate chart: {:?}", e);
            } else {
                println!("Candlestick chart with volatile days saved as {}", output_file);
            }
        }
        Err(e) => eprintln!("Error fetching stock data: {:?}", e),
    }
}

// fetch stock data using yahoo finance api crate
async fn fetch_stock_data(
    provider: &yahoo::YahooConnector,
    ticker: &str,
    interval: &str,
    range: &str,
) -> Result<Vec<Quote>, yahoo::YahooError> {
    let response = provider.get_quote_range(ticker, interval, range).await?;
    let yahoo_quotes = response.quotes()?;
    Ok(yahoo_quotes.iter().map(|q| q.into()).collect())
}

// generate a candlestick chart with volatile days highlighted in an HTML file
fn generate_candlestick_with_volatility_html(
    ticker: &str,
    quotes: &[Quote],
    output_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (dates, opens, highs, lows, closes) = convert_quotes_to_candlestick_data(quotes);

    let (v_dates, _, _, _, v_closes) = identify_volatile_days(&dates, &highs, &lows, &closes);

    let candlestick = Candlestick::new(dates.clone(), opens, highs, lows, closes)
        .name("Daily Prices");

    let volatile_trace = Scatter::new(v_dates, v_closes)
        .mode(Mode::Markers)
        .marker(Marker::new().size(10).color("#1f77b4")) // highlight volatile days blue
        .name("Volatile Days");

    let mut plot = Plot::new();
    plot.add_trace(candlestick);
    plot.add_trace(volatile_trace);

    plot.set_layout(
        Layout::new()
            .title(Title::new(format!("Candlestick Chart for {} (Last 6 Months)", ticker).as_str()))
            .x_axis(Axis::new().title(Title::new("Date")))
            .y_axis(Axis::new().title(Title::new("Price ($USD)")))
            .height(900),
    );

    plot.to_html(output_file);
    Ok(())
}

// yahoo quotes -> candlestick data
fn convert_quotes_to_candlestick_data(
    quotes: &[Quote],
) -> (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let dates: Vec<String> = quotes
        .iter()
        .map(|q| timestamp_to_date_string(q.timestamp))
        .collect();
    let opens: Vec<f64> = quotes.iter().map(|q| q.open).collect();
    let highs: Vec<f64> = quotes.iter().map(|q| q.high).collect();
    let lows: Vec<f64> = quotes.iter().map(|q| q.low).collect();
    let closes: Vec<f64> = quotes.iter().map(|q| q.close).collect();

    (dates, opens, highs, lows, closes)
}

/// day is volatile if (high - low > 2% of close)
fn identify_volatile_days(
    dates: &[String],
    highs: &[f64],
    lows: &[f64],
    closes: &[f64],
) -> (Vec<String>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut v_dates = Vec::new();
    let mut v_highs = Vec::new();
    let mut v_lows = Vec::new();
    let mut v_opens = Vec::new();
    let mut v_closes = Vec::new();

    for i in 0..dates.len() {
        if (highs[i] - lows[i]) / closes[i] > 0.02 {
            v_dates.push(dates[i].clone());
            v_highs.push(highs[i]);
            v_lows.push(lows[i]);
            v_opens.push(0.0); // place holder for open cuz unused
            v_closes.push(closes[i]);
        }
    }

    (v_dates, v_opens, v_highs, v_lows, v_closes)
}

// unix time -> readable format
fn timestamp_to_date_string(timestamp: i64) -> String {
    let datetime: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0)
        .single()
        .expect("Invalid timestamp"); // Handle invalid timestamps safely
    datetime.format("%Y-%m-%d").to_string() // Format the `DateTime` into a string
}

// quote data type
#[derive(Debug, Clone)]
struct Quote {
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

impl From<&yahoo::Quote> for Quote {
    fn from(quote: &yahoo::Quote) -> Self {
        Self {
            timestamp: quote.timestamp as i64,
            open: quote.open,
            high: quote.high,
            low: quote.low,
            close: quote.close,
        }
    }
}
