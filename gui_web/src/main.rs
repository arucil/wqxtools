#[macro_use]
extern crate rocket;

#[launch]
fn launch() -> _ {
  rocket::build()
}
