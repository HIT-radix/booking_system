use scrypto_test::prelude::*;

use booking_system::booking_system::booking_system_test::*;

#[test]
fn test_booking_system() -> Result<(), RuntimeError> {
    let mut env = TestEnvironment::new();
    env.disable_auth_module();
    let package_address =
        PackageFactory::compile_and_publish(this_package!(), &mut env, CompileProfile::Fast)?;

    // Create owner badge
    let badge_bucket = ResourceBuilder::new_fungible(OwnerRole::None)
        .divisibility(0)
        .mint_initial_supply(1, &mut env)?;
    let badge_address = badge_bucket.resource_address(&mut env)?;

    // Instantiate a BookingSystem component
    let mut booking_system = BookingSystem::new(
        badge_address,
        package_address,
        &mut env
    )?;

    // Create a user badge
    let user_badge_bucket1 = booking_system.new_user(
        &mut env
    )?;

    Ok(())
}
