use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::invoice)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Invoice{
    pub address: String,
    pub receiver: String,
    pub mnemonic: String,
    pub state: i32,
    pub value: f64,
    pub lifetime: i32,
    pub complete_action: i32
}
