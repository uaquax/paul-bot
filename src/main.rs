mod api_response;
mod area;
mod city;
mod command;
mod product;
mod state;

use api_response::ApiResponse;
use area::Area;
use city::City;
use dotenv::dotenv;
use product::Product;
use rand::Rng;
use serde_json::{json, Value};
use state::State;
use std::collections::HashMap;
use std::env;
use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};
use teloxide::{types::Message, Bot};

use crate::command::Command;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn initialize() {
    pretty_env_logger::init();
    dotenv().ok();

    let token = env::var("TOKEN").unwrap();

    env::set_var("TELOXIDE_TOKEN", token);
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>().branch(
        case![State::Start]
            .branch(case![Command::Start])
            .endpoint(start),
    );

    let message_handler = Update::filter_message().branch(command_handler);

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::Product].endpoint(receive_product))
        .branch(case![State::City { product }].endpoint(receive_city))
        .branch(case![State::Area { product, city }].endpoint(receive_area))
        .branch(
            case![State::ConfirmPurchase {
                product,
                city,
                area
            }]
            .endpoint(receive_purchase),
        );

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

#[tokio::main]
async fn main() {
    initialize();

    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

pub async fn get_products() -> Result<ApiResponse, reqwest::Error> {
    let url = "http://193.187.129.54:5000/api/content/product";
    let response = reqwest::get(url).await?.json::<ApiResponse>().await?;

    Ok(response)
}

pub async fn get_cities() -> Result<ApiResponse, reqwest::Error> {
    let url = "http://193.187.129.54:5000/api/content/city";
    let response = reqwest::get(url).await?.json::<ApiResponse>().await?;

    Ok(response)
}

pub async fn get_areas(city: String) -> Result<ApiResponse, reqwest::Error> {
    let url = format!("http://193.187.129.54:5000/api/content/area?id={}", city);
    let response = reqwest::get(url).await?.json::<ApiResponse>().await?;

    Ok(response)
}

pub async fn start(bot: Bot, dlg: MyDialogue, msg: Message) -> HandlerResult {
    let products = get_products().await.unwrap_or(ApiResponse {
        status: 200,
        msg: "".to_string(),
        description: "".to_string(),
        data: vec![json!({})],
    });

    let buttons: Vec<InlineKeyboardButton> = products
        .data
        .iter()
        .filter_map(|product| {
            let name = product.get("name").and_then(Value::as_str)?;
            let price = product.get("price").and_then(Value::as_str)?;
            let id = product.get("id").and_then(Value::as_str)?;
            Some(InlineKeyboardButton::callback(
                format!("{} - {}", name.to_string(), price.to_string()),
                format!("{}|{}", id, name),
            ))
        })
        .collect();

    let keyboard_rows: Vec<Vec<InlineKeyboardButton>> =
        buttons.chunks(2).map(|chunk| chunk.to_vec()).collect();

    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

    bot.send_message(msg.chat.id, "Выберите товар:")
        .reply_markup(keyboard)
        .await
        .unwrap();

    dlg.update(State::Product).await.unwrap();
    Ok(())
}

async fn receive_city(
    bot: Bot,
    dlg: MyDialogue,
    product: Product,
    q: CallbackQuery,
) -> HandlerResult {
    let city = q.data.unwrap();

    if city == "back" {
        if let Some(msg) = q.message {
            bot.delete_message(dlg.chat_id(), msg.id)
                .await
                .unwrap_or_default();
        }

        let products = get_products().await.unwrap_or(ApiResponse {
            status: 200,
            msg: "".to_string(),
            description: "".to_string(),
            data: vec![json!({})],
        });

        let buttons: Vec<InlineKeyboardButton> = products
            .data
            .iter()
            .filter_map(|product| {
                let name = product.get("name").and_then(Value::as_str)?;
                let price = product.get("price").and_then(Value::as_str)?;
                let id = product.get("id").and_then(Value::as_str)?;
                Some(InlineKeyboardButton::callback(
                    format!("{} - {}", name.to_string(), price.to_string()),
                    format!("{}|{}", id, name),
                ))
            })
            .collect();

        let keyboard_rows: Vec<Vec<InlineKeyboardButton>> =
            buttons.chunks(2).map(|chunk| chunk.to_vec()).collect();

        let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

        bot.send_message(dlg.chat_id(), "Выберите товар:")
            .reply_markup(keyboard)
            .await
            .unwrap();

        dlg.update(State::Product).await.unwrap();

        return Ok(());
    }

    let areas = get_areas(city.clone().split("|").nth(0).unwrap().to_string())
        .await
        .unwrap_or(ApiResponse {
            status: 200,
            msg: "".to_string(),
            description: "".to_string(),
            data: vec![json!({})],
        });
    let mut buttons: Vec<InlineKeyboardButton> = areas
        .data
        .iter()
        .filter_map(|area| {
            let name = area.get("name").and_then(Value::as_str)?;
            let id = area.get("id").and_then(Value::as_str)?;

            Some(InlineKeyboardButton::callback(
                name.to_string(),
                format!("{}|{}", id, name),
            ))
        })
        .collect();
    buttons.push(InlineKeyboardButton::callback("Назад", "back"));

    let keyboard_rows: Vec<Vec<InlineKeyboardButton>> =
        buttons.chunks(2).map(|chunk| chunk.to_vec()).collect();
    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

    bot.edit_message_text(
        dlg.chat_id(),
        q.message.as_ref().unwrap().id,
        "Выберите район:",
    )
    .reply_markup(keyboard)
    .await
    .unwrap();

    dlg.update(State::Area {
        product: product,
        city: City {
            id: city.split("|").nth(0).unwrap().to_string(),
            name: city.split("|").nth(1).unwrap().to_string(),
        },
    })
    .await
    .unwrap();

    Ok(())
}

async fn generate_id() -> String {
    let mut rng = rand::thread_rng();
    let random_value: u64 = rng.gen_range(1000000000..10000000000);

    random_value.to_string()
}

async fn receive_purchase(
    bot: Bot,
    dlg: MyDialogue,
    (product, city, area): (Product, City, Area),

    q: CallbackQuery,
) -> HandlerResult {
    let data = q.data.unwrap();

    if data == "confirm" {
        let url = "http://193.187.129.54:5000/api/purchase";
        let client = reqwest::Client::new();
        let mut map = HashMap::new();

        let order_id = generate_id().await;

        map.insert("city", city.id.clone());
        map.insert("product", product.id.clone());
        map.insert("area", area.id.clone());
        map.insert("orderid", order_id.clone());
        map.insert("userid", dlg.chat_id().0.to_string());

        client.post(url).json(&map).send().await.unwrap();

        bot.edit_message_text(
            dlg.chat_id(),
            q.message.as_ref().unwrap().id,
            format!(
                "Ваш заказ {}\n\nтовар:{}\nГород: {}\nРайон: {}\n\nС вами свяжется модератор",
                order_id, product.name, city.name, area.name
            ),
        )
        .await
        .unwrap();
    } else {
        bot.delete_message(dlg.chat_id(), q.message.as_ref().unwrap().id)
            .await
            .unwrap();
    }

    dlg.update(State::Start).await.unwrap();

    Ok(())
}

async fn receive_area(
    bot: Bot,
    dlg: MyDialogue,
    (product, city): (Product, City),

    q: CallbackQuery,
) -> HandlerResult {
    let area = q.data.unwrap();

    if area == "back" {
        if let Some(msg) = q.message {
            bot.delete_message(dlg.chat_id(), msg.id)
                .await
                .unwrap_or_default();
        }

        let areas = get_areas(city.id.clone()).await.unwrap_or(ApiResponse {
            status: 200,
            msg: "".to_string(),
            description: "".to_string(),
            data: vec![json!({})],
        });
        let mut buttons: Vec<InlineKeyboardButton> = areas
            .data
            .iter()
            .filter_map(|area| {
                let name = area.get("name").and_then(Value::as_str)?;
                let id = area.get("id").and_then(Value::as_str)?;

                Some(InlineKeyboardButton::callback(
                    name.to_string(),
                    format!("{}|{}", id, name),
                ))
            })
            .collect();
        buttons.push(InlineKeyboardButton::callback("Назад", "back"));

        let keyboard_rows: Vec<Vec<InlineKeyboardButton>> =
            buttons.chunks(2).map(|chunk| chunk.to_vec()).collect();
        let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

        bot.send_message(dlg.chat_id(), "Выберите район:")
            .reply_markup(keyboard)
            .await
            .unwrap();

        dlg.update(State::Area {
            product: product,
            city: City {
                id: city.id,
                name: city.name,
            },
        })
        .await
        .unwrap();

        return Ok(());
    }

    let keyboard = [
        InlineKeyboardButton::callback("Подтвердить", "confirm"),
        InlineKeyboardButton::callback("Отменить", "cancel"),
    ];

    bot.edit_message_text(
        dlg.chat_id(),
        q.message.as_ref().unwrap().id,
        "Отправить на рассмотрение",
    )
    .await
    .unwrap();

    bot.edit_message_reply_markup(dlg.chat_id(), q.message.as_ref().unwrap().id)
        .reply_markup(InlineKeyboardMarkup::new([keyboard]))
        .await
        .unwrap();

    dlg.update(State::ConfirmPurchase {
        product: product,
        city: city,
        area: Area {
            id: area.split("|").nth(0).unwrap().to_string(),
            name: area.split("|").nth(1).unwrap().to_string(),
        },
    })
    .await
    .unwrap();

    Ok(())
}

async fn receive_product(bot: Bot, dlg: MyDialogue, q: CallbackQuery) -> HandlerResult {
    let product = q.data.unwrap();

    let cities = get_cities().await.unwrap_or(ApiResponse {
        status: 200,
        msg: "".to_string(),
        description: "".to_string(),
        data: vec![json!({})],
    });
    let mut buttons: Vec<InlineKeyboardButton> = cities
        .data
        .iter()
        .filter_map(|city| {
            let name = city.get("name").and_then(Value::as_str)?;
            let id = city.get("id").and_then(Value::as_str)?;
            Some(InlineKeyboardButton::callback(
                name.to_string(),
                format!("{}|{}", id, name),
            ))
        })
        .collect();

    buttons.push(InlineKeyboardButton::callback("Назад", "back"));

    let keyboard_rows: Vec<Vec<InlineKeyboardButton>> =
        buttons.chunks(2).map(|chunk| chunk.to_vec()).collect();
    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

    bot.edit_message_text(
        dlg.chat_id(),
        q.message.as_ref().unwrap().id,
        "Выберите город:",
    )
    .await
    .unwrap();

    bot.edit_message_reply_markup(dlg.chat_id(), q.message.as_ref().unwrap().id)
        .reply_markup(keyboard)
        .await
        .unwrap();

    dlg.update(State::City {
        product: Product {
            id: product.split("|").nth(0).unwrap().to_string(),
            name: product.split("|").nth(1).unwrap().to_string(),
        },
    })
    .await
    .unwrap();

    Ok(())
}
