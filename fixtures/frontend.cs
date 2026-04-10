using Soul.Attributes;

public class CheckoutController
{
    [Soul("interaction.checkout.create-order", Role = "frontend")]
    public void CreateOrder()
    {
    }
}
