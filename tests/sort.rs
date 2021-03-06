use shipyard::error;
use shipyard::internal::iterators;
use shipyard::prelude::*;

#[test]
fn simple_sort() {
    let world = World::new();

    world.run::<(EntitiesMut, &mut usize), _, _>(|(mut entities, mut usizes)| {
        entities.add_entity(&mut usizes, 5);
        entities.add_entity(&mut usizes, 2);
        entities.add_entity(&mut usizes, 4);
        entities.add_entity(&mut usizes, 3);
        entities.add_entity(&mut usizes, 1);

        usizes.sort().unstable(Ord::cmp);

        let mut prev = 0;
        (&mut usizes).iter().for_each(|&mut x| {
            assert!(prev <= x);
            prev = x;
        });
    });
}

#[test]
fn tight_sort() {
    let world = World::new();
    let (mut entities, mut usizes, mut u32s) =
        world.borrow::<(EntitiesMut, &mut usize, &mut u32)>();

    (&mut usizes, &mut u32s).tight_pack();
    entities.add_entity((&mut usizes, &mut u32s), (10usize, 3u32));
    entities.add_entity((&mut usizes, &mut u32s), (5usize, 9u32));
    entities.add_entity((&mut usizes, &mut u32s), (1usize, 5u32));
    entities.add_entity((&mut usizes, &mut u32s), (3usize, 54u32));

    (&mut usizes, &mut u32s)
        .sort()
        .unstable(|(&x1, &y1), (&x2, &y2)| (x1 + y1 as usize).cmp(&(x2 + y2 as usize)));

    let mut prev = 0;
    (&mut usizes, &mut u32s)
        .iter()
        .for_each(|(&mut x, &mut y)| {
            assert!(prev <= x + y as usize);
            prev = x + y as usize;
        });
}

#[test]
fn loose_sort() {
    let world = World::new();
    let (mut entities, mut usizes, mut u32s) =
        world.borrow::<(EntitiesMut, &mut usize, &mut u32)>();

    (&mut usizes, &mut u32s).loose_pack();

    entities.add_entity((&mut usizes, &mut u32s), (10usize, 3u32));
    entities.add_entity((&mut usizes, &mut u32s), (5usize, 9u32));
    entities.add_entity((&mut usizes, &mut u32s), (1usize, 5u32));
    entities.add_entity((&mut usizes, &mut u32s), (3usize, 54u32));

    (&mut usizes, &mut u32s)
        .sort()
        .unstable(|(&x1, &y1), (&x2, &y2)| (x1 + y1 as usize).cmp(&(x2 + y2 as usize)));

    let mut prev = 0;
    (&mut usizes, &mut u32s)
        .iter()
        .for_each(|(&mut x, &mut y)| {
            assert!(prev <= x + y as usize);
            prev = x + y as usize;
        });
}

#[test]
fn tight_loose_sort() {
    let world = World::new();
    let (mut entities, mut usizes, mut u64s, mut u32s) =
        world.borrow::<(EntitiesMut, &mut usize, &mut u64, &mut u32)>();

    (&mut usizes, &mut u64s).tight_pack();
    LoosePack::<(u32,)>::loose_pack((&mut u32s, &mut usizes, &mut u64s));

    entities.add_entity((&mut usizes, &mut u64s), (3, 4));
    entities.add_entity((&mut usizes, &mut u64s, &mut u32s), (6, 7, 8));
    entities.add_entity((&mut usizes,), (5,));
    entities.add_entity((&mut usizes, &mut u64s, &mut u32s), (0, 1, 2));

    (&mut usizes, &mut u64s)
        .sort()
        .unstable(|(&x1, &y1), (&x2, &y2)| (x1 + y1 as usize).cmp(&(x2 + y2 as usize)));

    if let iterators::Iter3::Loose(mut iter) = (&usizes, &u64s, &u32s).iter() {
        assert_eq!(iter.next(), Some((&6, &7, &8)));
        assert_eq!(iter.next(), Some((&0, &1, &2)));
        assert_eq!(iter.next(), None);
    } else {
        panic!("not loose");
    }
    if let iterators::Iter2::Tight(mut iter) = (&usizes, &u64s).iter() {
        assert_eq!(iter.next(), Some((&0, &1)));
        assert_eq!(iter.next(), Some((&3, &4)));
        assert_eq!(iter.next(), Some((&6, &7)));
        assert_eq!(iter.next(), None);
    } else {
        panic!("not tight");
    }
    if let iterators::Iter2::NonPacked(mut iter) = (&usizes, &u32s).iter() {
        assert_eq!(iter.next(), Some((&6, &8)));
        assert_eq!(iter.next(), Some((&0, &2)));
        assert_eq!(iter.next(), None);
    }
}

#[test]
fn tight_sort_missing_storage() {
    let world = World::new();
    let (mut usizes, mut u64s) = world.borrow::<(&mut usize, &mut u64)>();

    (&mut usizes, &mut u64s).tight_pack();
    assert_eq!(
        usizes.sort().try_unstable(Ord::cmp).err(),
        Some(error::Sort::MissingPackStorage)
    );
}

#[test]
fn loose_sort_missing_storage() {
    let world = World::new();
    let (mut usizes, mut u64s) = world.borrow::<(&mut usize, &mut u64)>();

    (&mut usizes, &mut u64s).loose_pack();
    assert_eq!(
        usizes.sort().try_unstable(Ord::cmp).err(),
        Some(error::Sort::MissingPackStorage)
    );
}

#[test]
fn tight_sort_too_many_storages() {
    let world = World::new();
    let (mut usizes, mut u64s, mut u32s) = world.borrow::<(&mut usize, &mut u64, &mut u32)>();

    (&mut usizes, &mut u64s).tight_pack();
    assert_eq!(
        (&mut usizes, &mut u64s, &mut u32s)
            .sort()
            .try_unstable(|(&x1, &y1, &z1), (&x2, &y2, &z2)| {
                (x1 + y1 as usize + z1 as usize).cmp(&(x2 + y2 as usize + z2 as usize))
            })
            .err(),
        Some(error::Sort::TooManyStorages)
    );
}

#[test]
fn update_sort() {
    let world = World::new();
    let mut usizes = world.borrow::<&mut usize>();

    usizes.update_pack();
    assert_eq!(
        usizes.sort().try_unstable(Ord::cmp).err(),
        Some(error::Sort::MissingPackStorage)
    );
}
