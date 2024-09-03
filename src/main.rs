mod lib {
    pub mod one_to_one;
}

use lib::one_to_one;

fn main() {
    one_to_one::send("Hello world");
    one_to_one::receive();
}
