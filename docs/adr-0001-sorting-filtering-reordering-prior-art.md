# ADR-0001: Sorting, filtering, and reordering — prior art

- **Status:** Research (informational)
- **Date:** 2026-11-12
- **Related:** #215 (sorting is terrible), #217 (view-only sort), #218 (manual + priority only), #219 (drag in priority/filtered views)
- **Scope:** Jira, Trello, Things 3, Linear, GitHub Projects, Notion, Obsidian, Todoist

## Context

Issue #215 describes two problems that are easy to conflate:

1. The project-level sorting modes are mostly not useful, and the one useful mode (priority) does not compose with manual order.
2. Reordering is locked behind `sort mode === manual` and behind the absence of filters, even when the visible order is unambiguous.

The current implementation makes the coupling explicit. The board computes:

```ts
const narrowing = isTaskFilterNarrowing(filter);
const canReorder =
  optimisticProject !== undefined &&
  optimisticProject.task_sorting_mode === 'manual' &&
  !narrowing;
```

([`index.tsx:86-90`](../ui/src/views/projects/%5BprojectId%5D/index.tsx)). The narrowing predicate treats any state subset, tag, or search text as disqualifying ([`taskFilter.ts:62-68`](../ui/src/utils/taskFilter.ts)). Yet the position calculator already resolves a drop against the *full* destination section, not the visible subset ([`board.ts:233-242`](../ui/src/views/projects/%5BprojectId%5D/board.ts)). In other words, the data model already supports filtered reordering; the restriction is a product decision layered on top.

This ADR records how established tools handle the same questions. Behavior is time-bound; the Obsidian and Todoist findings reflect late 2026.

## Product findings

### Jira

Jira separates the order of work within a column from the column assignment itself. Boards group work items by status, while the `Rank` field supplies the order within a column and across the backlog. Atlassian's support documentation describes the mechanism directly:

> "By default, issues are **ordered by Rank** on Kanban Boards. Additionally, when drag-and-dropping issues to reorder them, what's actually happening is the **Rank** field on the issue is being updated."
>
> — [Atlassian, "Unable to drag and drop to reorder issues on a Kanban Board in Jira"](https://support.atlassian.com/jira/kb/unable-to-drag-and-drop-to-reorder-issues-on-a-kanban-board-in-jira/)

The same article states the condition under which reordering works: "Drag and drop ordering requires boards to be ordered by Rank." It then lists the failure modes:

> "If issues can't be reordered on a board, it could be because: 1. The board's filter query has been changed and no longer ends in 'ORDER BY Rank ASC' 2. The user attempting to reorder the issues does not have the permissions needed to modify Rank."
>
> — [Atlassian, same article](https://support.atlassian.com/jira/kb/unable-to-drag-and-drop-to-reorder-issues-on-a-kanban-board-in-jira/)

Rank is both persistent data and a view constraint. When a work item moves from the backlog into a sprint or work-in-progress on a board that uses a different ranking setting, the ranking is recalculated from the board's filter query: "If you're moving a work item from the backlog to an active sprint or your work-in-progress, and your board is using a ranking setting other than `ORDER BY Rank ASC`, the work item is automatically ranked based on the filter query of your board" ([Atlassian, "Rank a work item"](https://support.atlassian.com/jira-software-cloud/docs/rank-an-issue/)). Enabling ranking is likewise described in drag terms: "Enabling ranking allows you to rank work items on a board by drag-and-drop and create sprints on scrum boards" ([Atlassian, "Enable ranking"](https://support.atlassian.com/jira-software-cloud/docs/enable-ranking/)).

Jira also exposes list-view sorting as a separate control. The documentation describes sorting the list alphabetically by a field such as Summary, and notes a structural consequence: "When you sort your list view, child work items will stay with their parent" ([Atlassian, "Filter, sort, copy, and rank work items in the list"](https://support.atlassian.com/jira-software-cloud/docs/filter-sort-copy-and-rank-work-items-in-the-list/)). The list sort is a presentation operation; it does not replace the Rank field that boards use.

**Lesson.** Jira is the clearest example of a product that chooses a single persistent rank and then constrains the view to display it. The constraint is documented and produces a visible failure mode—cards snap back—rather than an ambiguous drag. It is coherent, but brittle: changing a filter's ordering can silently disable reordering.

### Trello

Trello has no persistent sort mode. Cards live in a manually ordered list, and "the most straightforward way to move a card is to simply click and drag the card up and down in the same list or between lists" ([Atlassian, "Moving cards or lists"](https://support.atlassian.com/trello/docs/moving-cards-or-lists/)).

Sorting is a destructive list action rather than a view setting. The documentation instructs the user to open the List actions menu, choose "Sort by," and pick a criterion such as alphabetical order, due date, or date created. It then states the consequence plainly:

> "There isn't a way to undo the sorting of a list other than to move cards back to their original position manually."
>
> — [Atlassian, "Sort cards in a list"](https://support.atlassian.com/trello/docs/sorting-lists/)

Filtering is a visibility operation. "Filtering on a board allows you to find cards with specific labels, members assigned to cards, due dates, and keywords," and "the filter will stay in place until you dismiss it, even if you leave the board or go to a different view such as the Calendar view" ([Atlassian, "Filtering for cards on a board"](https://support.atlassian.com/trello/docs/filtering-for-cards-on-a-board/)). Because there is no derived sort, filtering cannot conflict with the manual order: hidden cards are removed from view, and the remaining cards retain their positions.

Trello's Premium Table view is a projection rather than a second ordering system. The documentation describes it as a way to "see cards from a single board in list" and filter them by label, members, and due dates ([Atlassian, "Table view"](https://support.atlassian.com/trello/docs/single-board-table-view/)). Users report that row order follows the board: "The table view does not allow sorting by card title. It is sorted based on the order cards appear on your board (i.e. left-to-right by list and top-to-bottom for each list)" ([Atlassian Community, "Sort Table View by card Alphabetically"](https://community.atlassian.com/forums/Trello-questions/Sort-Table-View-by-card-Alphabetically/qaq-p/2968351)).

**Lesson.** Manual-only is the simplest model and eliminates sort/drag conflicts. Making sorting destructive is a poor fit for a list that is re-sorted frequently: the user must re-run the action whenever the underlying dates change, and the previous order is not recoverable except by manual repair.

### Things 3

Things 3 has no sort modes at all. Cultured Code's FAQ answers the question directly:

> "At this time, it's not possible to sort content alphabetically in Things. You can always manually re-arrange to-dos or your own lists in any order you prefer by dragging and dropping, though."
>
> — [Cultured Code, "Frequently Asked Questions"](https://culturedcode.com/things/support/articles/2967034/)

Order is manual across projects, areas, headings, Today, and other lists. The Move dialog and drag-and-drop change both the container and the position: the support article promises to show "how to move to-dos between lists or under headings and change the position of projects and areas, using the powerful Move dialog, drag & drop" ([Cultured Code, "Moving Items in Things"](https://culturedcode.com/things/support/articles/9651894/)).

Tags act as filters. The documentation describes the effect in visibility terms:

> "When you do this, the list will hide all the to-dos that don't have that tag, so that you can focus on the ones that do."
>
> — [Cultured Code, "Using Tags"](https://culturedcode.com/things/support/articles/2803581/)

Because there is no derived sort, filtering cannot introduce an alternative order. The visible items are the manual order with non-matching items removed.

**Lesson.** A task manager can ship with zero sort modes if manual order is good. "Manual only" is a legitimate product position, not an incomplete one. Things reduces the pressure for derived sorts through rich manual structure—projects, areas, headings—and by not offering cross-project saved-query views.

### Linear

Linear treats ordering as a view display option, alongside layout and grouping. The documentation states: "Manual ordering is default for board views and can be selected in most list views" ([Linear, "Display options"](https://linear.app/docs/display-options)). The available orderings include "Status, Manual, Priority, Last created, Last updated, Due date, and Link count," and "you can also reverse the sort order of your issues, except when sorting manually" ([Linear, "Display options"](https://linear.app/docs/display-options)).

Manual ordering is not merely personal presentation. Linear's documentation is explicit:

> "To move issues around, select them with your mouse and drag them to a new position. Manual ordering is unique in that it will update the manual order for everyone in the workspace."
>
> — [Linear, "Display options"](https://linear.app/docs/display-options)

Grouping and ordering interact. "Grouping settings will affect whether you can order issues manually" ([Linear, "Display options"](https://linear.app/docs/display-options)). When grouping is active, a cross-group drag is a property change:

> "You can drag and drop issues between each grouping and it will automatically adopt the properties of that grouping."
>
> — [Linear, "Display options"](https://linear.app/docs/display-options)

Filters, by contrast, refine which issues appear without replacing the ordering model: "Filters help narrow a view to the issues most relevant to the work at hand. They can be used across issue lists to focus on specific assignees, statuses, labels, projects, cycles, and more" ([Linear, "Filters"](https://linear.app/docs/filters)). Display options can be saved "as personal preferences or as the default display options on that page for your workspace" ([Linear, "Display options"](https://linear.app/docs/display-options)).

**Lesson.** Linear is the closest modern analogue to scry's target model: view-scoped sort, manual as the board default, filters orthogonal to order, and a cross-group drag that changes the grouped property. The difference is that Linear's groups are first-class columns or lanes, so the destination property is visually obvious.

### GitHub Projects

GitHub Projects applies filtering, sorting, and grouping per view. A view can be configured independently, and "when you make changes to a view—for example, sorting, reordering, filtering, or grouping the data in a view—a dot is displayed next to the view name" ([GitHub, "Managing your views"](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/managing-your-views)).

Board columns are generated from a single-select or iteration field. A column drag changes the field value:

> "You can drag individual or multiple items from column to column and the value of those items will adjust to match the column you drag them to."
>
> — [GitHub, "Customizing the board layout"](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)

The same page documents the sort/drag interaction with unusual precision:

> "When a board is sorted, you cannot manually reorder items within a column. You can continue to move items from column to column."
>
> — [GitHub, "Customizing the board layout"](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)

Grouping works the same way: "When items are grouped, if you drag an item to a new group, the value of that group is applied" ([GitHub, "Customizing the board layout"](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)). Sorting itself is a field operation: "You can sort items by a field value," with optional secondary sorts and direction changes ([GitHub, "Customizing the board layout"](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)).

Manual order is shared rather than view-scoped. Users report the trade-off:

> "Currently, when we change the order of issues in a view A it also changes all other views on the board."
>
> — [GitHub Community, "Projects Beta discussion #15178"](https://github.com/orgs/community/discussions/15178)

The same discussion explains the consequence for unsorted views: "if you choose no sorting (manual) on both views, you change the ordering in both views, when you change order in one" ([GitHub Community](https://github.com/orgs/community/discussions/15178)).

**Lesson.** GitHub Projects is the closest structural analogue to scry's current implementation. A sorted board disables within-column reorder but keeps cross-column drag as a property change. It also demonstrates a useful split: manual order is shared data, while sort is per-view.

### Notion

Notion separates data from presentation more aggressively than any other product surveyed. View settings are local to a view:

> "Each database view has its own settings. Settings applied to one database view won't be applied across all other database views automatically."
>
> — [Notion, "Database views, filters, sorts & groups"](https://www.notion.com/help/views-filters-and-sorts)

Filters, sorts, and groups are all view-level. "Sort: Sort by a property to change the order that pages are displayed in" ([Notion, "Database views, filters, sorts & groups"](https://www.notion.com/help/views-filters-and-sorts)). Settings can be personal or shared: "You can choose to `Save for everyone` if you want the filter to be applied for everyone in the database view" ([Notion, "Database views, filters, sorts & groups"](https://www.notion.com/help/views-filters-and-sorts)).

Board view groups pages by a property: "Board view groups your database pages by a specific property" ([Notion, "Board view"](https://www.notion.com/help/boards)). Cards can be moved within and between groups: "To move cards up and down or between columns, click, hold, and drag." Columns can be reordered: "To rearrange columns, click and hold on a heading, then drag left or right" ([Notion, "Board view"](https://www.notion.com/help/boards)).

Manual ordering is itself a sort option. Notion's guidance for ordering status groups is to "click `Sort` and select `Manual` to be able to drag your categories into the desired order" ([Notion, "Status property gives you clarity on task progress"](https://www.notion.com/help/guides/status-property-gives-clarity-on-tasks)). The resulting model is clean: a non-manual sort derives the displayed order; a manual sort makes vertical drag meaningful; and a cross-column drag changes the grouped property regardless.

**Lesson.** Notion demonstrates that sort, filter, and group can be entirely view-scoped without users perceiving the app as losing their order. It also demonstrates the separation between manual vertical order and group-property change that #219 proposes for priority.

### Obsidian

Bases, Obsidian's database layer, uses views with filters, sorts, groups, and formulas. The view's Display menu exposes Sort and Group, and the documentation describes properties, search, and the Display menu as view-level controls ([Obsidian, "Views"](https://help.obsidian.md/bases/views)).

The native Kanban view arrived in Obsidian 1.14:

> "Bases now has a kanban view. Organize your notes into columns and drag cards between groups. You can also reorder and hide groups in any Bases view using the new Group menu."
>
> — [Obsidian, "1.14 Desktop changelog"](https://obsidian.md/changelog/2026-10-05-desktop-v1.14.4/)

The important limitation is vertical order within a column. A detailed feature request from October 2026 states the behavior:

> "Within a column, order is whatever the view's Sort setting says - a drop only changes the group property."
>
> — [Obsidian Forum, "Bases Kanban: manual card ordering within a column"](https://forum.obsidian.md/t/bases-kanban-manual-card-ordering-within-a-column-drag-and-drop-persisted-to-a-property/119092)

The same post explains the user impact:

> "I drag across columns to change state, but to prioritize what might be 10–15 notes in a column, I drag up and down. That's one of my most frequent uses of a Kanban board, and today it's not possible."
>
> — [Obsidian Forum, same request](https://forum.obsidian.md/t/bases-kanban-manual-card-ordering-within-a-column-drag-and-drop-persisted-to-a-property/119092)

The proposed workaround is a numeric order property: "When a card is dropped within a column, write an order property to the note the same way the group property is written on a cross-column drop" ([Obsidian Forum](https://forum.obsidian.md/t/bases-kanban-manual-card-ordering-within-a-column-drag-and-drop-persisted-to-a-property/119092)). Community plugins add the missing behavior; one describes itself as: "Drag cards between columns to update frontmatter automatically. Drag to reorder cards and columns" ([Obsidian Forum, "Bases Kanban Plugin"](https://forum.obsidian.md/t/bases-kanban-plugin/109179)).

**Lesson.** A brand-new, well-resourced Kanban implementation omitted manual in-column ordering and immediately generated strong feedback. Manual vertical order is table stakes for a board, even when grouping is the dominant interaction. The low-tech fallback—a numeric order property plus a view sort—also shows why a fractional position is the better primitive.

### Todoist

Todoist has per-view layout, sorting, grouping, and filters. Sort options include Manual, Name, Date, Date added, Deadline, and Priority; group options include Date, Date added, Deadline, Priority, and Label ([Todoist, "Sort or group tasks in Todoist"](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)). Sorting and grouping are personal view settings: "Sorting or grouping options aren't shared with others in a project, so you can tweak your views and find which one works best without disrupting anyone" ([Todoist, same article](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)).

Todoist has the most restrictive drag rules of the products surveyed. Grouping disables manual reorder:

> "If a grouping option is applied to a project or view, you can't drag or manually reorder your tasks."
>
> — [Todoist, "Sort or group tasks in Todoist"](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)

Filtered and label views are more restrictive still:

> "Manual sorting is only available in projects and in the Today and Upcoming views. Filter and label views always use their default sorting order and can't be manually reordered, even when no grouping option is applied."
>
> — [Todoist, same article](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)

The structural reason is that filters and labels are saved queries, not project containers ([Todoist, "Introduction to filters"](https://www.todoist.com/help/todoist/features/introduction-to-filters-V98wIH)). A Todoist support reply in a user thread states the same limitation: "manual sorting isn't supported inside Filters or Labels, even if you set sorting to 'manual' in the View settings. Only project views and the Today/Upcoming views support drag-and-drop" ([r/todoist, "no manual sorting in filters???"](https://www.reddit.com/r/todoist/comments/1p71djn/no_manual_sorting_in_filters/)).

Within a project, ordering is scoped further. "Tasks are sorted within their own sections," and "the task view always shows sub-tasks in their manual order, and sorting doesn't change that saved order" ([Todoist, "Sort or group tasks in Todoist"](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)). The board layout uses sections as columns: "Add and rearrange sections. Use sections to represent phases of the project. Drag tasks to relevant sections. Sort or filter tasks across the board" ([Todoist, "Use the board layout in Todoist"](https://www.todoist.com/help/todoist/features/use-the-board-layout-in-todoist-AiAVsyEI)).

**Lesson.** Todoist is the anti-pattern that #215 describes: filters disable reorder, and users dislike it. But Todoist has a structural excuse that scry does not. Its filters and labels can combine tasks from multiple projects, so there is no single manual coordinate space. Scry's positions are scoped to a section within one project, so a filtered scry view still has a well-defined manual order.

## Comparison matrix

Product names link to the primary source used for the row.

| Product | Persistent order | Sort location | Filter effect on order | Drag under a non-manual sort | Cross-group drag |
| --- | --- | --- | --- | --- | --- |
| [Jira](https://support.atlassian.com/jira/kb/unable-to-drag-and-drop-to-reorder-issues-on-a-kanban-board-in-jira/) | `Rank` field | Board/view ordering; board must be ordered by Rank for rank drag | Hides items; Rank unchanged | Disabled or snaps back unless ordered by Rank | Between columns changes status, not Rank |
| [Trello](https://support.atlassian.com/trello/docs/sorting-lists/) | List position | No persistent sort; one-shot destructive list action | Hides cards; list order unchanged | Manual drag always | Between lists moves the card |
| [Things 3](https://culturedcode.com/things/support/articles/2967034/) | Manual position | No sort modes | Hides items; manual order unchanged | Not applicable | Between projects, areas, or headings |
| [Linear](https://linear.app/docs/display-options) | Manual order, workspace-shared | Per-view ordering | Hides items; manual order remains available | Manual ordering limited by grouping | Adopts the destination group's property |
| [GitHub Projects](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout) | Manual order, reported shared across unsorted views | Per-view sort | Hides items; manual order unchanged | Sorted board disables within-column reorder; cross-column still works | Applies the destination column or group value |
| [Notion](https://www.notion.com/help/views-filters-and-sorts) | Manual order as a view sort option | Per-view sort | Hides items; manual sort remains selectable | Manual sort required for vertical drag | Applies the destination group property |
| [Obsidian Bases](https://forum.obsidian.md/t/bases-kanban-manual-card-ordering-within-a-column-drag-and-drop-persisted-to-a-property/119092) | View sort; no native in-column manual order | Per-view sort and group | Hides items; sort order remains | No native in-column manual drag in Kanban | Applies the destination group property |
| [Todoist](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb) | Manual order per project section | Per-view sort and group | Filter and label views have no manual order | Grouping disables drag; filters and labels never manual | Board drag between sections |
| [Scry today](../ui/src/views/projects/%5BprojectId%5D/index.tsx) | Task `position` per section | Project-level mode | Disables reorder entirely | Disabled unless manual and unfiltered | Section change only |
| [Scry target](../crates/scry-core/src/ordering.rs) | Task `position` per section | URL/view sort | Hides items; reorder remains available | Priority drag also changes priority | Section and priority as needed |

## Cross-cutting patterns

### 1. Order as data and order as view are different things

Every mature product separates a persistent order from a display order, but they draw the line in different places:

- **Persistent order is the only order:** Trello, Things.
- **Persistent rank plus a view that must use it:** Jira.
- **Persistent manual order plus independent view sorts:** Linear, GitHub Projects, Notion, Todoist, Obsidian Bases.

Scry currently has a persistent order (`position`) and a project-level view sort. The position is already the right primitive. The problem is that the view sort is stored as project data and that drag is coupled to the one mode where the view sort happens to equal the persistent order.

### 2. Grouping is the hinge for cross-container drag

In every product that supports dragging between groups, the drag changes the grouped property:

- Linear: "automatically adopt the properties of that grouping" ([Linear](https://linear.app/docs/display-options)).
- GitHub Projects: "the value of those items will adjust to match the column you drag them to" ([GitHub](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)).
- Notion: board columns are groups, and cards are dragged between them ([Notion](https://www.notion.com/help/boards)).
- Obsidian Bases Kanban: a cross-column drop writes the group property ([Obsidian Forum](https://forum.obsidian.md/t/bases-kanban-manual-card-ordering-within-a-column-drag-and-drop-persisted-to-a-property/119092)).
- Todoist: "Drag tasks to relevant sections" ([Todoist](https://www.todoist.com/help/todoist/features/use-the-board-layout-in-todoist-AiAVsyEI)).

This is the pattern #219 follows when a priority drag changes the task's priority. The precedent is strong, but it depends on the group being visually legible. In Linear, GitHub, Notion, and Todoist, the group is a column or lane with a header. In scry, priority is a comparator rather than a separate column, so the destination has to be made legible in the UI.

### 3. Filtering is orthogonal to ordering, with one structural exception

Trello, Things 3, Linear, GitHub Projects, Notion, and Obsidian treat filtering as visibility. Hiding items does not invalidate the relative order of the visible items. Todoist is the exception, and its documentation makes the restriction explicit: "Filter and label views always use their default sorting order and can't be manually reordered, even when no grouping option is applied" ([Todoist](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)).

The difference is structural. Todoist filters and labels are saved queries that can span projects. Scry's positions are per section and its filters never span projects. A filtered scry view is a subsequence of a well-defined manual order, so reordering in it is representable. The evidence therefore does not support copying Todoist's restriction.

### 4. Four ways to handle drag under a sort

The surveyed products use four strategies:

1. **Disable position drag.** GitHub Projects: "When a board is sorted, you cannot manually reorder items within a column" ([GitHub](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout)). Jira requires rank ordering for drag ([Jira KB](https://support.atlassian.com/jira/kb/unable-to-drag-and-drop-to-reorder-issues-on-a-kanban-board-in-jira/)). Todoist disables drag under grouping ([Todoist](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)).
2. **Change the grouped property.** Linear, GitHub Projects board columns, Notion boards, Todoist board sections, and Obsidian Kanban.
3. **Destructive one-shot sort.** Trello ([Trello](https://support.atlassian.com/trello/docs/sorting-lists/)).
4. **Manual position as a secondary sort key.** Not common as an explicit product model, but it is the natural extension of "manual within group" behavior. Scry's #218 proposes this for priority.

### 5. View-scoped sort is the modern default

Notion states that "each database view has its own settings" ([Notion](https://www.notion.com/help/views-filters-and-sorts)). GitHub Projects marks a view as modified when its sort, filter, grouping, or order changes ([GitHub](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/managing-your-views)). Linear saves display options as personal or workspace defaults ([Linear](https://linear.app/docs/display-options)). Todoist keeps sorting and grouping personal ([Todoist](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)). Obsidian Bases exposes Sort and Group in the view's Display menu ([Obsidian](https://help.obsidian.md/bases/views)).

Jira is the notable exception: Rank is global data, and the board is expected to display it. Trello and Things avoid the question by having no persistent sort. For a local, single-user tool, moving sort into the URL is a coherent version of the view-scoped model. The trade-off is that the CLI and API lose a stored default; manual becomes the baseline order.

### 6. Destructive sorts and silent disables are the two recurring failure modes

- Trello's one-shot sort is surprising because it permanently rewrites list order and has no undo beyond manual repair ([Trello](https://support.atlassian.com/trello/docs/sorting-lists/)).
- Jira's snap-back and Todoist's filtered-view restriction are surprising because the affordance appears available or the reason is not visible ([Jira KB](https://support.atlassian.com/jira/kb/unable-to-drag-and-drop-to-reorder-issues-on-a-kanban-board-in-jira/); [Todoist](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb)).

If scry enables drag in a sorted or filtered view, the resulting property change (priority) should be visible immediately. If it ever disables drag, the reason should be stated in the UI rather than inferred.

## Implications for scry

### #217: view-scoped sort

View-scoped sort is mainstream. Notion, GitHub Projects, Linear, Todoist, and Obsidian Bases all treat sorting as a view concern ([Notion](https://www.notion.com/help/views-filters-and-sorts); [GitHub](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/managing-your-views); [Linear](https://linear.app/docs/display-options); [Todoist](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb); [Obsidian](https://help.obsidian.md/bases/views)). Removing the project field entirely is more aggressive than those products, most of which retain a workspace or project default, but it is coherent for a local single-user tool. Manual as the default matches Things, Trello, Linear boards, and unsorted GitHub views.

The main risk is the CLI and API losing a stored default. The prior art does not require a stored default; Jira's Rank is the exception because rank is data, not a view preference. Manual as the baseline order is consistent with the products that have no persistent sort.

### #218: manual + priority, with position as the priority tiebreak

Manual-only is a legitimate product position (Things, Trello), and priority is the only derived order with real semantic content in scry's current model. Alphabetical and created are convenience orders, and there is no due date to make a date sort meaningful ([`models.rs:348-360`](../crates/scry-core/src/models.rs)). Using manual position as the priority tiebreak approximates "group by priority, manual within group" without introducing a separate group field. It is a novel combination in the surveyed set, but it follows directly from Notion's and Linear's "manual within group" behavior ([Notion](https://www.notion.com/help/views-filters-and-sorts); [Linear](https://linear.app/docs/display-options)).

### #219: drag in priority and filtered views

Cross-group drag changing the grouped property has strong precedent ([Linear](https://linear.app/docs/display-options); [GitHub](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout); [Notion](https://www.notion.com/help/boards); [Todoist](https://www.todoist.com/help/todoist/features/use-the-board-layout-in-todoist-AiAVsyEI); [Obsidian](https://forum.obsidian.md/t/bases-kanban-manual-card-ordering-within-a-column-drag-and-drop-persisted-to-a-property/119092)). Scry's priority is a comparator rather than a separate column, so the implementation must make the target priority legible during the drag; otherwise the property change will feel like a side effect.

Filtered reorder is justified. The Todoist restriction is structural, not a general law, and scry's section-scoped positions give a filtered view a well-defined coordinate space ([Todoist](https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb); [`board.ts:233-242`](../ui/src/views/projects/%5BprojectId%5D/board.ts)). Hiding tasks should not disable reorder.

Manual order should remain shared project data. GitHub Projects' model is the closest match: manual order is shared across unsorted views, while sort is per-view ([GitHub Community](https://github.com/orgs/community/discussions/15178)). That is also scry's model after #217, and it is the right one to preserve. Only the sort should be view-scoped; the positions should not be.

## Decision

No implementation decision is recorded in this ADR. This document establishes the prior-art baseline for the decisions tracked in #217 (view-scoped sort), #218 (manual + priority, position tiebreak), and #219 (drag in priority and filtered views). The behavior for priority drags is not open: #219 specifies that a drag to another priority group changes the dragged task's priority, and the evidence above supports that choice.

## Consequences

- The implementation of #217–#219 should preserve section-scoped fractional positions as the persistent order.
- Sort should be view-scoped; manual order should remain shared project data.
- Filters should not gate reorder; they hide items and leave their positions alone.
- A cross-group drag in priority view should be treated as a property change, consistent with Linear, GitHub Projects, Notion, Todoist, and Obsidian Bases.
- Destructive one-shot sorting (Trello) and silent drag disabling (Jira, Todoist) are explicitly not the model.
- The implementation should make the target priority legible during the drag rather than add drag exceptions.

## References

### Jira

- Atlassian. "Unable to drag and drop to reorder issues on a Kanban Board in Jira." 2025. <https://support.atlassian.com/jira/kb/unable-to-drag-and-drop-to-reorder-issues-on-a-kanban-board-in-jira/>
- Atlassian. "Rank a work item." Jira Cloud documentation. <https://support.atlassian.com/jira-software-cloud/docs/rank-an-issue/>
- Atlassian. "Enable ranking." Jira Cloud documentation. <https://support.atlassian.com/jira-software-cloud/docs/enable-ranking/>
- Atlassian. "Filter, sort, copy, and rank work items in the list." 2025. <https://support.atlassian.com/jira-software-cloud/docs/filter-sort-copy-and-rank-work-items-in-the-list/>

### Trello

- Atlassian. "Sort cards in a list." 2022. <https://support.atlassian.com/trello/docs/sorting-lists/>
- Atlassian. "Moving cards or lists." 2022. <https://support.atlassian.com/trello/docs/moving-cards-or-lists/>
- Atlassian. "Filtering for cards on a board." <https://support.atlassian.com/trello/docs/filtering-for-cards-on-a-board/>
- Atlassian. "Table view." <https://support.atlassian.com/trello/docs/single-board-table-view/>
- Atlassian Community. "Sort Table View by card Alphabetically." 2025. <https://community.atlassian.com/forums/Trello-questions/Sort-Table-View-by-card-Alphabetically/qaq-p/2968351>

### Things 3

- Cultured Code. "Frequently Asked Questions." <https://culturedcode.com/things/support/articles/2967034/>
- Cultured Code. "Using Tags." <https://culturedcode.com/things/support/articles/2803581/>
- Cultured Code. "Moving Items in Things." <https://culturedcode.com/things/support/articles/9651894/>

### Linear

- Linear. "Display options." <https://linear.app/docs/display-options>
- Linear. "Filters." <https://linear.app/docs/filters>
- Linear. "Board Ordering." Changelog, 2022-08-18. <https://linear.app/changelog/2022-08-18-board-ordering>
- Linear. "Multi-team boards & manual ordering." Changelog, 2021-08-27. <https://linear.app/changelog/2021-08-27-multi-team-boards-and-manual-ordering>

### GitHub Projects

- GitHub. "Customizing the board layout." <https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-board-layout>
- GitHub. "Customizing the table layout." <https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/customizing-the-table-layout>
- GitHub. "Managing your views." <https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/managing-your-views>
- GitHub Community. "[Projects Beta] Ability to sort issues independently in different views?" Discussion #15178. <https://github.com/orgs/community/discussions/15178>

### Notion

- Notion. "Database views, filters, sorts & groups." <https://www.notion.com/help/views-filters-and-sorts>
- Notion. "Board view." <https://www.notion.com/help/boards>
- Notion. "The status property gives you clarity on task progress." <https://www.notion.com/help/guides/status-property-gives-clarity-on-tasks>

### Obsidian

- Obsidian. "Views." Obsidian Help. <https://help.obsidian.md/bases/views>
- Obsidian. "Obsidian 1.14 Desktop changelog." 2026-10-05. <https://obsidian.md/changelog/2026-10-05-desktop-v1.14.4/>
- Obsidian Forum. "Bases Kanban: manual card ordering within a column / drag and drop (persisted to a property)." 2026-10-06. <https://forum.obsidian.md/t/bases-kanban-manual-card-ordering-within-a-column-drag-and-drop-persisted-to-a-property/119092>
- Obsidian Forum. "Bases Kanban Plugin." 2025-12-20. <https://forum.obsidian.md/t/bases-kanban-plugin/109179>

### Todoist

- Todoist. "Sort or group tasks in Todoist." Updated 2026-10-28. <https://www.todoist.com/help/articles/sort-or-group-tasks-in-todoist-WFWD0hrb>
- Todoist. "Customize views in Todoist." 2026-09-15. <https://www.todoist.com/help/todoist/features/customize-views-in-todoist-AoHhBxFdZ>
- Todoist. "Use the board layout in Todoist." 2026-11-05. <https://www.todoist.com/help/todoist/features/use-the-board-layout-in-todoist-AiAVsyEI>
- Todoist. "Default sorting order in Todoist." 2025-02-11. <https://www.todoist.com/help/todoist/features/default-sorting-order-in-todoist-mqmgerY7>
- Todoist. "Introduction to filters." 2026-11-05. <https://www.todoist.com/help/todoist/features/introduction-to-filters-V98wIH>
- r/todoist. "no manual sorting in filters???" 2025-12-12. <https://www.reddit.com/r/todoist/comments/1p71djn/no_manual_sorting_in_filters/>

### Scry source references

- [`crates/scry-core/src/models.rs`](../crates/scry-core/src/models.rs) — sorting enum, project/task models.
- [`crates/scry-core/src/ordering.rs`](../crates/scry-core/src/ordering.rs) — comparator.
- [Board page](../ui/src/views/projects/%5BprojectId%5D/index.tsx) — reorder guard (`index.tsx:86-90`).
- [`ui/src/utils/taskFilter.ts`](../ui/src/utils/taskFilter.ts) — narrowing predicate.
- [Board drop-position logic](../ui/src/views/projects/%5BprojectId%5D/board.ts) — drop position calculation (`board.ts:233-242`).
- [`ui/src/utils/projectOperations.ts`](../ui/src/utils/projectOperations.ts) — UI-created project defaults.
- [`migrations/20261031164943_init.up.sql`](../migrations/20261031164943_init.up.sql) — project sort column and seed.
