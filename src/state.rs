#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    Product,
    City {
        product: String,
    },
    Area {
        product: String,
        city: String,
    },
    ConfirmPurchase {
        product: String,
        city: String,
        area: String,
    },
}
