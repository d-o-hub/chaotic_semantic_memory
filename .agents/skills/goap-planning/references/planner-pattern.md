# Planner Pattern

## A* Skeleton

```python
from heapq import heappop, heappush

def plan_actions(current_state, goal, actions):
    open_set = [(0, current_state, [])]
    seen = set()

    while open_set:
        cost, state, path = heappop(open_set)
        key = freeze(state)
        if key in seen:
            continue
        seen.add(key)

        if satisfies_goal(state, goal):
            return path

        for action in actions:
            if can_execute(action, state):
                new_state = apply_effects(action, state)
                new_cost = cost + action.cost
                heappush(open_set, (new_cost, new_state, path + [action]))

    return None
```

## State Update Template

```yaml
world_state:
  action_last_completed: create_module_hyperdim
  hyperdim_module_created: true
  loc_count:
    hyperdim.rs: 387
```
