use crate::{area::Area, city::City, product::Product};

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    Product,
    City {
        product: Product,
    },
    Area {
        product: Product,
        city: City,
    },
    ConfirmPurchase {
        product: Product,
        city: City,
        area: Area,
    },
}
