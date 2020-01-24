use shipyard::prelude::*;

struct Parent {
    num_children: usize,
    first_child: EntityId,
}

struct Child {
    parent: EntityId,
    prev: EntityId,
    next: EntityId,
}

trait Hierarchy {
    // Removes the child status of an entity.
    fn detach(&mut self, id: EntityId);

    // Attaches an entity as a child to a given parent entity.
    fn attach(&mut self, id: EntityId, parent: EntityId);
    fn attach_new(&mut self, parent: EntityId) -> EntityId;
    fn remove(&mut self, id: EntityId);
    fn remove_all(&mut self, id: EntityId);
    fn sort_children_by<F>(&mut self, id: EntityId, compare: F)
    where
        F: FnMut(&EntityId, &EntityId) -> std::cmp::Ordering;
}

impl Hierarchy for (EntitiesViewMut<'_>, ViewMut<'_, Parent>, ViewMut<'_, Child>) {
    fn detach(&mut self, id: EntityId) {
        // remove the Child component - if nonexistent, do nothing
        if let Some(child) = self.2.remove(id) {
            // retrieve and update Parent component from ancestor
            let parent = &mut self.1[child.parent];
            parent.num_children -= 1;

            if parent.num_children == 0 {
                // if the number of children is zero, the Parent component must be removed
                self.1.remove(child.parent);
            } else {
                // the ancestor still has children, and we have to change some linking
                // check if we have to change first_child
                if parent.first_child == id {
                    parent.first_child = child.next;
                }
                // remove the detached child from the sibling chain
                self.2[child.prev].next = child.next;
                self.2[child.next].prev = child.prev;
            }
        }
    }
    fn attach(&mut self, id: EntityId, parent: EntityId) {
        // the entity we want to attach might already be attached to another parent
        self.detach(id);

        // either the designated parent already has a Parent component – and thus one or more children
        if let Some(p) = (&mut self.1).get(parent) {
            // increase the parent's children counter
            p.num_children += 1;

            // get the ids of the new previous and next siblings of our new child
            let prev = self.2[p.first_child].prev;
            let next = p.first_child;

            // change the linking
            self.2[prev].next = id;
            self.2[next].prev = id;

            // add the Child component to the new entity
            self.0
                .add_component(&mut self.2, Child { parent, prev, next }, id);
        } else {
            // in this case our designated parent is missing a Parent component
            // we don't need to change any links, just insert both components
            self.0.add_component(
                &mut self.2,
                Child {
                    parent,
                    prev: id,
                    next: id,
                },
                id,
            );
            self.0.add_component(
                &mut self.1,
                Parent {
                    num_children: 1,
                    first_child: id,
                },
                parent,
            );
        }
    }
    fn attach_new(&mut self, parent: EntityId) -> EntityId {
        let id = self.0.add_entity((), ());
        self.attach(id, parent);
        id
    }
    fn remove(&mut self, id: EntityId) {
        self.detach(id);
        let children = (&self.1, &self.2).children(id).collect::<Vec<_>>();
        for child_id in children {
            self.detach(child_id);
        }
        self.1.remove(id);
    }
    fn remove_all(&mut self, id: EntityId) {
        for child_id in (&self.1, &self.2).children(id).collect::<Vec<_>>() {
            self.remove_all(child_id);
        }
        self.remove(id);
    }
    fn sort_children_by<F>(&mut self, id: EntityId, mut compare: F)
    where
        F: FnMut(&EntityId, &EntityId) -> std::cmp::Ordering,
    {
        let mut children = (&self.1, &self.2).children(id).collect::<Vec<EntityId>>();
        if children.len() > 1 {
            children.sort_by(|a, b| compare(a, b));
            // set first_child in Parent component
            self.1[id].first_child = children[0];
            // loop through children and relink them
            for i in 0..children.len() - 1 {
                self.2[children[i]].next = children[i + 1];
                self.2[children[i + 1]].prev = children[i];
            }
            self.2[children[0]].prev = *children.last().unwrap();
            self.2[*children.last().unwrap()].next = children[0];
        }
    }
}

struct ChildrenIter<C> {
    get_child: C,
    cursor: (EntityId, usize),
}

impl<'a, C> Iterator for ChildrenIter<C>
where
    C: GetComponent<Out = &'a Child> + Copy,
{
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
            let ret = self.cursor.0;
            self.cursor.0 = self.get_child.get(self.cursor.0).unwrap().next;
            Some(ret)
        } else {
            None
        }
    }
}

struct AncestorIter<C> {
    get_child: C,
    cursor: EntityId,
}

impl<'a, C> Iterator for AncestorIter<C>
where
    C: GetComponent<Out = &'a Child> + Copy,
{
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.get_child.get(self.cursor).map(|child| {
            self.cursor = child.parent;
            child.parent
        })
    }
}

struct DescendantsIter<P, C> {
    get_parent: P,
    get_child: C,
    cursors: Vec<(EntityId, usize)>,
}

impl<'a, P, C> Iterator for DescendantsIter<P, C>
where
    P: GetComponent<Out = &'a Parent> + Copy,
    C: GetComponent<Out = &'a Child> + Copy,
{
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cursor) = self.cursors.last_mut() {
            if cursor.1 > 0 {
                cursor.1 -= 1;
                let ret = cursor.0;
                cursor.0 = self.get_child.get(cursor.0).unwrap().next;
                if let Some(parent) = self.get_parent.get(ret) {
                    self.cursors.push((parent.first_child, parent.num_children));
                }
                Some(ret)
            } else {
                self.cursors.pop();
                self.next()
            }
        } else {
            None
        }
    }
}
trait HierarchyIter<'a, P, C> {
    fn ancestors(&self, id: EntityId) -> AncestorIter<C>;
    fn children(&self, id: EntityId) -> ChildrenIter<C>;
    fn descendants(&self, id: EntityId) -> DescendantsIter<P, C>;
}

impl<'a, P, C> HierarchyIter<'a, P, C> for (P, C)
where
    P: GetComponent<Out = &'a Parent> + Copy,
    C: GetComponent<Out = &'a Child> + Copy,
{
    fn ancestors(&self, id: EntityId) -> AncestorIter<C> {
        AncestorIter {
            get_child: self.1,
            cursor: id,
        }
    }

    fn children(&self, id: EntityId) -> ChildrenIter<C> {
        ChildrenIter {
            get_child: self.1,
            cursor: self
                .0
                .get(id)
                .map_or((id, 0), |parent| (parent.first_child, parent.num_children)),
        }
    }

    fn descendants(&self, id: EntityId) -> DescendantsIter<P, C> {
        DescendantsIter {
            get_parent: self.0,
            get_child: self.1,
            cursors: self.0.get(id).map_or_else(Vec::new, |parent| {
                vec![(parent.first_child, parent.num_children)]
            }),
        }
    }
}

#[test]
fn basic() {
    let world = World::new();

    let mut hierarchy = world.borrow::<(EntitiesMut, &mut Parent, &mut Child)>();

    let root1 = hierarchy.0.add_entity((), ());
    let root2 = hierarchy.0.add_entity((), ());

    let e1 = hierarchy.attach_new(root1);
    let _e2 = hierarchy.attach_new(e1);
    let e3 = hierarchy.attach_new(e1);
    let _e4 = hierarchy.attach_new(e3);

    hierarchy.attach(e3, root2);
}

#[test]
fn test_hierarchy() {
    let world = World::new();

    let mut hierarchy = world.borrow::<(EntitiesMut, &mut Parent, &mut Child)>();

    let root1 = hierarchy.0.add_entity((), ());
    let root2 = hierarchy.0.add_entity((), ());

    let e1 = hierarchy.attach_new(root1);
    let e2 = hierarchy.attach_new(e1);
    let e3 = hierarchy.attach_new(e1);
    let e4 = hierarchy.attach_new(e3);

    hierarchy.attach(e3, root2);

    let e5 = hierarchy.attach_new(e3);

    assert!((&hierarchy.1, &hierarchy.2)
        .children(e3)
        .eq([e4, e5].iter().cloned()));

    assert!((&hierarchy.1, &hierarchy.2)
        .ancestors(e4)
        .eq([e3, root2].iter().cloned()));

    assert!((&hierarchy.1, &hierarchy.2)
        .descendants(root1)
        .eq([e1, e2].iter().cloned()));

    assert!((&hierarchy.1, &hierarchy.2)
        .descendants(root2)
        .eq([e3, e4, e5].iter().cloned()));

    hierarchy.detach(e1);

    assert!((&hierarchy.1, &hierarchy.2).descendants(root1).eq(None));
    assert!((&hierarchy.1, &hierarchy.2).ancestors(e1).eq(None));
    assert!((&hierarchy.1, &hierarchy.2)
        .children(e1)
        .eq([e2].iter().cloned()));

    hierarchy.remove(e1);

    assert!((&hierarchy.1, &hierarchy.2).children(e1).eq(None));

    hierarchy.remove_all(root2);

    assert!((&hierarchy.1, &hierarchy.2).descendants(root2).eq(None));
    assert!((&hierarchy.1, &hierarchy.2).descendants(e3).eq(None));
    assert!((&hierarchy.1, &hierarchy.2).ancestors(e5).eq(None));
}

#[test]
fn test_sorting() {
    let world = World::new();

    let (mut hierarchy, mut usizes) =
        world.borrow::<((EntitiesMut, &mut Parent, &mut Child), &mut usize)>();

    let root = hierarchy.0.add_entity((), ());

    let e0 = hierarchy.attach_new(root);
    let e1 = hierarchy.attach_new(root);
    let e2 = hierarchy.attach_new(root);
    let e3 = hierarchy.attach_new(root);
    let e4 = hierarchy.attach_new(root);

    hierarchy.0.add_component(&mut usizes, 7, e0);
    hierarchy.0.add_component(&mut usizes, 5, e1);
    hierarchy.0.add_component(&mut usizes, 6, e2);
    hierarchy.0.add_component(&mut usizes, 1, e3);
    hierarchy.0.add_component(&mut usizes, 3, e4);

    assert!((&hierarchy.1, &hierarchy.2)
        .children(root)
        .eq([e0, e1, e2, e3, e4].iter().cloned()));

    hierarchy.sort_children_by(root, |a, b| usizes[*a].cmp(&usizes[*b]));

    assert!((&hierarchy.1, &hierarchy.2)
        .children(root)
        .eq([e3, e4, e1, e2, e0].iter().cloned()));
}