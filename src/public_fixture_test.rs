use crate::world::{ItemCache, MobCache, RoomCache};

#[test]
fn public_fixture_loads_without_private_game_content() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/public-data");

    let room = RoomCache::with_data_dir(root.join("map"))
        .get_room("공개시험", "1")
        .expect("public room fixture");
    assert_eq!(room.read().unwrap().display_name, "공개 시험장");

    let mut mobs = MobCache::with_data_dir(root.join("mob"));
    let mob = mobs
        .load_mob("공개시험", "훈련인형")
        .expect("public mob fixture");
    assert_eq!(mob.name, "훈련인형");

    let mut items = ItemCache::with_data_dir(root.join("item"));
    let item = items.load_item("시험목검").expect("public item fixture");
    assert_eq!(item.name, "시험목검");
}
