// Standard Uses
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::ast::listener::ListenerController;
use crate::shaping::patch::{Actions, Patch};
use crate::shaping::patch::trace::{ActionTrace, Trace};
use crate::shaping::operation::actions::{Action, ActionOf};
use crate::shaping::operation::actions::builder::BuilderAction;
use crate::shaping::operation::actions::maker::MakerAction;
use crate::shaping::operation::actions::replacer::ReplacerAction;
use crate::shaping::operation::actions::indexer::{ListAction, SwitchAction};

// External Uses
use indextree::{Arena, NodeId};
use miette::Result;


#[allow(unused)]
pub struct PatchController {
    pub patch: Rc<Patch>,
    pub actions: Arena<ActionOf>,
    pub listener_controller: ListenerController,
    pub trace: Option<RefCell<Trace>>,
    paths: HashMap<NodeId, String>
}

#[allow(unused)]
impl PatchController {
    pub fn with(patch: Rc<Patch>) -> Self {
        Self::with_listener(patch, Default::default())
    }

    pub fn with_listener(patch: Rc<Patch>, listener_controller: ListenerController) -> Self {
        let (actions, paths) = Self::make_actions(patch.clone());

        Self { patch, actions, listener_controller, trace: None, paths }
    }

    pub fn traced(patch: Rc<Patch>, limit: usize) -> Self {
        Self { trace: Some(RefCell::new(Trace::new(limit))), ..Self::with(patch) }
    }

    pub fn path(&self, id: NodeId) -> Option<&str> {
        self.paths.get(&id).map(String::as_str)
    }

    pub fn tracing(&self) -> bool {
        self.trace.is_some()
    }

    pub(crate) fn record(&self, id: NodeId, record: impl FnOnce(&mut ActionTrace, usize)) {
        let (Some(trace), Some(path)) = (&self.trace, self.paths.get(&id)) else { return };
        let mut trace = trace.borrow_mut();
        let limit = trace.limit;

        record(trace.actions.entry(path.clone()).or_default(), limit);
    }

    fn make_actions(patch: Rc<Patch>) -> (Arena<ActionOf>, HashMap<NodeId, String>) {
        let mut arena = Arena::<ActionOf>::new();
        let mut paths = HashMap::new();
        Self::make_inner_actions(&mut arena, &mut paths, &patch.actions, None);
        (arena, paths)
    }

    fn make_inner_actions(
        arena: &mut Arena<ActionOf>, paths: &mut HashMap<NodeId, String>, actions: &Actions, parent: Option<&NodeId>
    ) {
        let parent_path = parent.and_then(|parent| paths.get(parent)).cloned().unwrap_or_default();
        let path = |name: &str| if parent_path.is_empty() { name.to_string() } else { format!("{parent_path}/{name}") };

        if let Some(builders) = &actions.builders {
            for builder in builders.iter() {
                let action = ActionOf::BuilderAction(
                    BuilderAction::with_ref(builder.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().builder_mut().unwrap().post_new(id);
                paths.insert(id, path(&builder.name));

                if let Some(parent) = parent { parent.append(id, arena); }

                Self::make_inner_actions(arena, paths, &builder.actions, Some(&id));
            }
        }

        if let Some(replacers) = &actions.replacers {
            for (index, replacer) in replacers.iter().enumerate() {
                let action = ActionOf::ReplacerAction(
                    ReplacerAction::with_ref(replacer.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().replacer_mut().unwrap().post_new(id);
                paths.insert(id, path(&format!("replacer #{}", index + 1)));

                if let Some(parent) = parent { parent.append(id, arena); }

                Self::make_inner_actions(arena, paths, &replacer.actions, Some(&id));
            }
        }

        if let Some(makers) = &actions.makers {
            for maker in makers.iter() {
                let action = ActionOf::MakerAction(
                    MakerAction::with_ref(maker.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().maker_mut().unwrap().post_new(id);
                paths.insert(id, path(&maker.name));

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }

        if let Some(lists) = &actions.lists {
            for list in lists.iter() {
                let action = ActionOf::ListAction(
                    ListAction::with_ref(list.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().list_mut().unwrap().post_new(id);
                paths.insert(id, path(&list.name));

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }

        if let Some(switchers) = &actions.switchers {
            for switch in switchers.iter() {
                let action = ActionOf::SwitchAction(
                    SwitchAction::with_ref(switch.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().switch_mut().unwrap().post_new(id);
                paths.insert(id, path(&switch.name));

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }
    }

    pub fn apply(&mut self, backend: &dyn LanguageBackend, source: &str) -> Result<String> {
        let replacer_ids: Vec<NodeId> = self.actions.iter()
            .filter(|node| node.parent().is_none())
            .filter_map(|node| node.get().replacer().ok().map(|replacer| *replacer.self_id()))
            .collect();

        let mut locations = vec![];
        collect_locations(&self.patch.actions, &mut locations);

        let mut source = source.to_string();

        for id in replacer_ids {
            let replacer = self.actions[id].get().replacer()?.replacer().clone();
            let triggers = backend.locate(&source, &replacer.location)?;

            let mut located = HashMap::new();
            for location in &locations {
                located.insert(location.as_str(), backend.locate(&source, location)?);
            }

            self.listener_controller.location_contents = located.iter()
                .map(|(location, ranges)| (
                    location.to_string(),
                    ranges.iter().map(|range| source[range.clone()].to_string()).collect()
                ))
                .collect();

            let mut edits: Vec<(Range<usize>, String)> = vec![];
            for trigger in triggers {
                let mut content = HashMap::new();
                let mut target = None;

                for (location, ranges) in &located {
                    let Some(range) = scoped_range(ranges, &trigger) else { continue };

                    if *location == replacer.target_location() {
                        target = Some(range.clone());
                    }
                    content.insert(location.to_string(), source[range.clone()].to_string());
                }

                let Some(target) = target else { continue };
                if edits.iter().any(|(range, _)| *range == target) { continue }

                self.listener_controller.location_content = content;
                let Some(replaced) = self.actions[id].get().process(self) else { continue };

                edits.push((target, replaced));
            }

            edits.sort_by_key(|(range, _)| range.start);

            let mut applied: Vec<(Range<usize>, String)> = vec![];
            for edit in edits {
                if applied.last().is_some_and(|(last, _)| edit.0.start < last.end) { continue }
                applied.push(edit);
            }

            for (range, text) in applied.into_iter().rev() {
                source.replace_range(range, &text);
            }
        }

        Ok(source)
    }

    pub fn visit(&mut self) {
        for action in self.actions.iter() {
            let action = action.get();

            match action {
                ActionOf::BuilderAction(b) => {
                    let result = b.process(&self);
                }
                ActionOf::ReplacerAction(r) => {
                    let result = r.process(&self);
                }
                _ => {}
            };
        }
    }

    pub fn find_action<'t>(
        &'t self, name: &str, controller: &'t PatchController
    ) -> Option<&ActionOf> {
        for action in self.actions.iter() {
            let action: &ActionOf = action.get();

            if action.name() == name { return Some(action) }

            if let Some(name) = action.find_inner(name, controller) {
                return Some(name)
            }
        }

        None
    }

    pub fn find_action_mut(&mut self, name: &str) -> Option<&mut ActionOf> {
        for action in self.actions.iter_mut() {
            let action = action.get_mut();

            if action.name() == name { return Some(action) }
        }

        None
    }

    pub fn possible_rules(&self) -> Vec<&'static str> {
        vec!["*"]
    }
}


fn collect_locations(actions: &Actions, locations: &mut Vec<String>) {
    for builder in actions.builders.iter().flatten() {
        locations.push(builder.location.clone());
        locations.extend(builder.reference_location.clone());
        collect_locations(&builder.actions, locations);
    }

    for replacer in actions.replacers.iter().flatten() {
        locations.push(replacer.location.clone());
        locations.push(replacer.reference_location.clone());
        collect_locations(&replacer.actions, locations);
    }

    locations.retain(|location| !location.is_empty());
    locations.sort();
    locations.dedup();
}

fn scoped_range<'r>(ranges: &'r [Range<usize>], trigger: &Range<usize>) -> Option<&'r Range<usize>> {
    ranges.iter()
        .filter(|range| range.start <= trigger.start && trigger.end <= range.end)
        .min_by_key(|range| range.end - range.start)
        .or_else(|| ranges.iter().find(|range| trigger.start <= range.start && range.end <= trigger.end))
}
