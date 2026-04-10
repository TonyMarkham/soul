use soul_attributes::soul;

#[soul(id = "interaction.checkout.create-order", role = "backend")]
fn create_order() {}

fn main() {
    create_order();
}
