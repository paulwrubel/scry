# Service / Store call graph

Derived from `src/service.rs` (`ProjectService`), `crates/scry-core/src/store.rs` (the `Store` trait), `crates/scry-core/src/backup.rs`, and the REST handlers in `src/server/handlers.rs`. `PostgresStore` is the only implementor of `Store`, and `ProjectService` is the only caller of `Store` apart from the backup module.

## How to read these diagrams

- Solid arrow (`-->`): a call across the boundary into the `Store` trait.
- Dashed arrow (`-.->`): an internal `ProjectService` call, including delegations into `scry_core::backup`.
- Thick arrow (`==>`): a REST handler calling into the service. The route is in the handler's label. CLI commands call the same service methods and are not drawn.
- A node label containing arrows, such as `append_position -> last_task_position -> tasks_in_section`, is a collapsed call chain drawn as one node so the diagram stays legible; its edges leave the collapsed node for the store calls made at the end of the chain. The arrow direction is written with the ASCII sequence `->`.
- `*_by_id` and `*_in_project` service methods are drawn only where a handler or another service method calls them.

## 1. Tasks - mutations

Creating, updating, moving, closing, opening, deleting, and duplicating tasks. Every mutation reads the task, changes fields, and writes the whole row back with one `Store::update_task`; `duplicate_task` copies the row through `Store::create_task`.

```mermaid
flowchart LR
  subgraph http["HTTP handlers"]
    h_create_task["create_task  POST /projects/{project_id}/tasks"]
    h_update_task["update_task  PUT /tasks/{id}"]
    h_move_task["move_task  POST /tasks/{id}/move"]
    h_close_task["close_task  POST /tasks/{id}/close"]
    h_open_task["open_task  POST /tasks/{id}/open"]
    h_delete_task["delete_task  DELETE /tasks/{id}"]
  end

  subgraph svc["ProjectService"]
    create_task[create_task]
    create_task_in_project[create_task_in_project]
    duplicate_task[duplicate_task]
    update_task[update_task]
    update_task_by_id[update_task_by_id]
    move_task_to_section[move_task_to_section]
    move_task_to_section_by_id[move_task_to_section_by_id]
    close_task[close_task]
    close_task_by_id[close_task_by_id]
    open_task[open_task]
    open_task_by_id[open_task_by_id]
    delete_task[delete_task]
    delete_task_by_id[delete_task_by_id]
    create_section[create_section]
  end

  subgraph helpers["internal helpers"]
    resolve_section[resolve_section]
    append_position["append_position -> last_task_position -> tasks_in_section"]
    tasks_in_section[tasks_in_section]
    require_task_in_project[require_task_in_project]
    require_task[require_task]
    require_project[require_project]
    require_section_in_project[require_section_in_project]
    validated_section_name[validated_section_name]
  end

  subgraph store["Store"]
    st_create_task[create_task]
    st_update_task[update_task]
    st_delete_task[delete_task]
    st_get_task_by_id[get_task_by_id]
    st_get_project_by_id[get_project_by_id]
    st_get_section_by_id[get_section_by_id]
    st_get_all_tasks_by_project_id[get_all_tasks_by_project_id]
    st_get_all_tasks_by_section_id[get_all_tasks_by_section_id]
    st_create_section[create_section]
    st_get_all_sections_by_project_id[get_all_sections_by_project_id]
  end

  h_create_task ==> create_task_in_project
  h_update_task ==> update_task_by_id
  h_move_task ==> move_task_to_section_by_id
  h_close_task ==> close_task_by_id
  h_open_task ==> open_task_by_id
  h_delete_task ==> delete_task_by_id

  create_task_in_project -.-> require_project
  create_task_in_project -.-> create_task
  update_task_by_id -.-> require_task
  update_task_by_id -.-> require_project
  update_task_by_id -.-> update_task
  move_task_to_section_by_id -.-> require_task
  move_task_to_section_by_id -.-> require_project
  move_task_to_section_by_id -.-> move_task_to_section
  close_task_by_id -.-> require_task
  close_task_by_id -.-> require_project
  close_task_by_id -.-> close_task
  open_task_by_id -.-> require_task
  open_task_by_id -.-> require_project
  open_task_by_id -.-> open_task
  delete_task_by_id -.-> require_task
  delete_task_by_id -.-> require_project
  delete_task_by_id -.-> delete_task

  create_task -.-> resolve_section
  create_task -.-> append_position
  create_task --> st_create_task
  duplicate_task -.-> require_task_in_project
  duplicate_task -.-> tasks_in_section
  duplicate_task -.-> require_section_in_project
  duplicate_task --> st_create_task
  update_task -.-> require_task_in_project
  update_task -.-> resolve_section
  update_task -.-> append_position
  update_task --> st_update_task
  move_task_to_section -.-> require_task_in_project
  move_task_to_section -.-> require_section_in_project
  move_task_to_section -.-> resolve_section
  move_task_to_section -.-> append_position
  move_task_to_section --> st_update_task
  close_task -.-> require_task_in_project
  close_task --> st_update_task
  open_task -.-> require_task_in_project
  open_task --> st_update_task
  delete_task -.-> require_task_in_project
  delete_task --> st_delete_task
  resolve_section -.-> require_section_in_project
  resolve_section -.-> create_section
  create_section -.-> validated_section_name
  create_section --> st_get_all_sections_by_project_id
  create_section --> st_create_section
  validated_section_name --> st_get_all_sections_by_project_id
  append_position --> st_get_all_tasks_by_section_id
  append_position --> st_get_all_tasks_by_project_id
  require_task_in_project --> st_get_task_by_id
  require_section_in_project --> st_get_section_by_id
  require_task --> st_get_task_by_id
  require_project --> st_get_project_by_id
  tasks_in_section --> st_get_all_tasks_by_section_id

  classDef storeNode fill:#eef3fb,stroke:#5b7fae,stroke-width:1px
  class st_create_task,st_update_task,st_delete_task,st_get_task_by_id,st_get_project_by_id,st_get_section_by_id,st_get_all_tasks_by_project_id,st_get_all_tasks_by_section_id,st_create_section,st_get_all_sections_by_project_id storeNode
```

## 2. Tasks - reads and handles

Listing tasks by project or section, single-task lookup, and the handle machinery. Handles are computed at display time by the CLI, so they have no handler layer here; `get_task_by_handle` turns a typed prefix into a task.

```mermaid
flowchart LR
  subgraph http["HTTP handlers"]
    h_get_task["get_task  GET /tasks/{id}"]
    h_get_tasks_by_project["get_tasks_by_project  GET /projects/{project_id}/tasks"]
    h_get_tasks_by_section["get_tasks_by_section  GET /sections/{id}/tasks"]
  end

  subgraph svc["ProjectService"]
    get_task[get_task]
    list_tasks_by_project_id[list_tasks_by_project_id]
    list_tasks_by_section_id[list_tasks_by_section_id]
    list_task_handles[list_task_handles]
    get_task_handle[get_task_handle]
    get_task_by_handle[get_task_by_handle]
  end

  subgraph helpers["internal helpers"]
    require_task[require_task]
  end

  subgraph store["Store"]
    st_get_task_by_id[get_task_by_id]
    st_get_all_tasks_by_project_id[get_all_tasks_by_project_id]
    st_get_all_tasks_by_section_id[get_all_tasks_by_section_id]
    st_get_all_task_ids[get_all_task_ids]
    st_find_task_ids_by_handle[find_task_ids_by_handle]
  end

  h_get_task ==> get_task
  h_get_tasks_by_project ==> list_tasks_by_project_id
  h_get_tasks_by_section ==> list_tasks_by_section_id

  get_task --> st_get_task_by_id
  list_tasks_by_project_id --> st_get_all_tasks_by_project_id
  list_tasks_by_section_id --> st_get_all_tasks_by_section_id
  list_task_handles --> st_get_all_task_ids
  get_task_handle --> st_get_all_task_ids
  get_task_by_handle --> st_find_task_ids_by_handle
  get_task_by_handle -.-> require_task
  require_task --> st_get_task_by_id

  classDef storeNode fill:#eef3fb,stroke:#5b7fae,stroke-width:1px
  class st_get_task_by_id,st_get_all_tasks_by_project_id,st_get_all_tasks_by_section_id,st_get_all_task_ids,st_find_task_ids_by_handle storeNode
```

## 3. Notes

Adding, listing, fetching, updating, and deleting notes. Notes have no handler for handle resolution; `get_note_handle` and `get_note_by_handle` exist for the CLI and for future note-prefix entry points.

```mermaid
flowchart LR
  subgraph http["HTTP handlers"]
    h_create_note["create_note  POST /tasks/{id}/notes"]
    h_get_note["get_note  GET /notes/{id}"]
    h_get_notes_by_task["get_notes_by_task  GET /tasks/{id}/notes"]
    h_get_notes_by_project["get_notes_by_project  GET /projects/{project_id}/notes"]
    h_update_note["update_note  PUT /notes/{id}"]
    h_delete_note["delete_note  DELETE /notes/{id}"]
  end

  subgraph svc["ProjectService"]
    add_note[add_note]
    add_note_to_task[add_note_to_task]
    get_note[get_note]
    list_notes_by_project_id[list_notes_by_project_id]
    list_notes_by_task_id[list_notes_by_task_id]
    update_note[update_note]
    delete_note[delete_note]
    get_note_handle[get_note_handle]
    get_note_by_handle[get_note_by_handle]
  end

  subgraph helpers["internal helpers"]
    require_task_in_project[require_task_in_project]
    require_task[require_task]
    require_project[require_project]
    require_note[require_note]
  end

  subgraph store["Store"]
    st_create_note[create_note]
    st_get_note_by_id[get_note_by_id]
    st_get_all_notes_by_project_id[get_all_notes_by_project_id]
    st_get_all_notes_by_task_id[get_all_notes_by_task_id]
    st_update_note[update_note]
    st_delete_note[delete_note]
    st_find_note_ids_by_handle[find_note_ids_by_handle]
    st_get_all_note_ids[get_all_note_ids]
    st_get_task_by_id[get_task_by_id]
    st_get_project_by_id[get_project_by_id]
  end

  h_create_note ==> add_note
  h_get_note ==> get_note
  h_get_notes_by_task ==> list_notes_by_task_id
  h_get_notes_by_project ==> list_notes_by_project_id
  h_update_note ==> update_note
  h_delete_note ==> delete_note

  add_note -.-> require_task
  add_note -.-> require_project
  add_note -.-> add_note_to_task
  add_note_to_task -.-> require_task_in_project
  add_note_to_task --> st_create_note
  list_notes_by_project_id --> st_get_all_notes_by_project_id
  list_notes_by_task_id --> st_get_all_notes_by_task_id
  get_note --> st_get_note_by_id
  update_note --> st_update_note
  delete_note --> st_delete_note
  get_note_handle --> st_get_all_note_ids
  get_note_by_handle --> st_find_note_ids_by_handle
  get_note_by_handle -.-> require_note
  require_note --> st_get_note_by_id
  require_task_in_project --> st_get_task_by_id
  require_project --> st_get_project_by_id
  require_task --> st_get_task_by_id

  classDef storeNode fill:#eef3fb,stroke:#5b7fae,stroke-width:1px
  class st_create_note,st_get_note_by_id,st_get_all_notes_by_project_id,st_get_all_notes_by_task_id,st_update_note,st_delete_note,st_find_note_ids_by_handle,st_get_all_note_ids,st_get_task_by_id,st_get_project_by_id storeNode
```

## 4. Sections

Section creation, lookup, listing, updates, deletion, and merging. Name lookups are exact, and a miss fails with `SectionNotFound`, which is why the lookup reaches `require_project` to name the project in the error. Merge is service-level: it moves each source task with `Store::update_task` and then deletes the source, so it is not atomic.

```mermaid
flowchart LR
  subgraph http["HTTP handlers"]
    h_create_section["create_section  POST /projects/{project_id}/sections"]
    h_get_section["get_section  GET /sections/{id}"]
    h_get_section_by_name["get_section_by_name  GET /projects/{project_id}/sections/by-name/{name}"]
    h_get_sections["get_sections  GET /projects/{project_id}/sections"]
    h_update_section["update_section  PUT /sections/{id}"]
    h_delete_section["delete_section  DELETE /sections/{id}"]
    h_merge_section["merge_section  POST /sections/{id}/merge"]
  end

  subgraph svc["ProjectService"]
    create_section[create_section]
    create_section_in_project[create_section_in_project]
    get_section[get_section]
    get_section_by_name[get_section_by_name]
    list_sections_by_project_id[list_sections_by_project_id]
    update_section[update_section]
    delete_section[delete_section]
    delete_section_by_id[delete_section_by_id]
    merge_sections[merge_sections]
    merge_sections_by_id[merge_sections_by_id]
  end

  subgraph helpers["internal helpers"]
    validated_section_name[validated_section_name]
    require_section_in_project[require_section_in_project]
    require_section[require_section]
    require_project[require_project]
    tasks_in_section[tasks_in_section]
  end

  subgraph store["Store"]
    st_create_section[create_section]
    st_get_section_by_id[get_section_by_id]
    st_get_section_by_name[get_section_by_name]
    st_get_all_sections_by_project_id[get_all_sections_by_project_id]
    st_get_all_tasks_by_section_id[get_all_tasks_by_section_id]
    st_update_section[update_section]
    st_update_task[update_task]
    st_delete_section[delete_section]
    st_get_project_by_id[get_project_by_id]
  end

  h_create_section ==> create_section_in_project
  h_get_section ==> get_section
  h_get_section_by_name ==> get_section_by_name
  h_get_sections ==> list_sections_by_project_id
  h_update_section ==> update_section
  h_delete_section ==> delete_section_by_id
  h_merge_section ==> merge_sections_by_id

  create_section_in_project -.-> require_project
  create_section_in_project -.-> create_section
  delete_section_by_id -.-> require_section
  delete_section_by_id -.-> require_project
  delete_section_by_id -.-> delete_section
  merge_sections_by_id -.-> require_section
  merge_sections_by_id -.-> require_project
  merge_sections_by_id -.-> merge_sections

  create_section -.-> validated_section_name
  create_section --> st_get_all_sections_by_project_id
  create_section --> st_create_section
  get_section_by_name --> st_get_section_by_name
  get_section_by_name -.-> require_project
  get_section --> st_get_section_by_id
  list_sections_by_project_id --> st_get_all_sections_by_project_id
  update_section -.-> require_section
  update_section -.-> require_project
  update_section -.-> validated_section_name
  update_section --> st_update_section
  delete_section -.-> require_section_in_project
  delete_section --> st_delete_section
  merge_sections -.-> require_section_in_project
  merge_sections -.-> tasks_in_section
  merge_sections --> st_update_task
  merge_sections --> st_delete_section
  validated_section_name --> st_get_all_sections_by_project_id
  require_section_in_project --> st_get_section_by_id
  require_section --> st_get_section_by_id
  require_project --> st_get_project_by_id
  tasks_in_section --> st_get_all_tasks_by_section_id

  classDef storeNode fill:#eef3fb,stroke:#5b7fae,stroke-width:1px
  class st_create_section,st_get_section_by_id,st_get_section_by_name,st_get_all_sections_by_project_id,st_get_all_tasks_by_section_id,st_update_section,st_update_task,st_delete_section,st_get_project_by_id storeNode
```

## 5. Projects

Project creation, listing, lookup, snapshot, update, and deletion. The read methods are thin pass-throughs to `Store`. `create_project` is the only writer that also creates sections, and only when given a `ProjectSetup::Template`. The update handler reads the project before writing it because `update_project` needs the row it is patching.

```mermaid
flowchart LR
  subgraph http["HTTP handlers"]
    h_create_project["create_project  POST /projects"]
    h_get_project["get_project  GET /projects/{id}"]
    h_get_project_by_name["get_project_by_name  GET /projects/by-name/{name}"]
    h_get_projects["get_projects  GET /projects"]
    h_get_project_snapshot["get_project_snapshot  GET /projects/{id}/snapshot"]
    h_update_project["update_project  PUT /projects/{id}"]
    h_delete_project_by_id["delete_project_by_id  DELETE /projects/{id}"]
  end

  subgraph svc["ProjectService"]
    create_project[create_project]
    get_project[get_project]
    get_project_by_name[get_project_by_name]
    list_projects[list_projects]
    project_snapshot[project_snapshot]
    update_project[update_project]
    delete_project_by_id[delete_project_by_id]
  end

  subgraph helpers["internal helpers"]
    require_project[require_project]
  end

  subgraph store["Store"]
    st_create_project[create_project]
    st_create_section[create_section]
    st_get_project_by_id[get_project_by_id]
    st_get_project_by_name[get_project_by_name]
    st_get_all_projects[get_all_projects]
    st_get_project_snapshot[get_project_snapshot]
    st_update_project[update_project]
    st_delete_project_by_id[delete_project_by_id]
  end

  h_create_project ==> create_project
  h_get_project ==> get_project
  h_get_project_by_name ==> get_project_by_name
  h_get_projects ==> list_projects
  h_get_project_snapshot ==> project_snapshot
  h_update_project ==> get_project
  h_update_project ==> update_project
  h_delete_project_by_id ==> delete_project_by_id

  create_project --> st_create_project
  create_project --> st_create_section
  get_project --> st_get_project_by_id
  get_project_by_name --> st_get_project_by_name
  list_projects --> st_get_all_projects
  project_snapshot --> st_get_project_snapshot
  update_project --> st_update_project
  delete_project_by_id -.-> require_project
  delete_project_by_id --> st_delete_project_by_id
  require_project --> st_get_project_by_id

  classDef storeNode fill:#eef3fb,stroke:#5b7fae,stroke-width:1px
  class st_create_project,st_create_section,st_get_project_by_id,st_get_project_by_name,st_get_all_projects,st_get_project_snapshot,st_update_project,st_delete_project_by_id storeNode
```

## 6. Backup and import

`export` and `plan_import` delegate into `scry_core::backup`, which reads through `&dyn Store`; `import` applies a plan directly. Only `import` has a handler — export and plan creation are CLI-only, since the plan is produced by the client before it is posted.

```mermaid
flowchart LR
  subgraph http["HTTP handlers"]
    h_import["import  POST /import"]
  end

  subgraph svc["ProjectService"]
    export[export]
    plan_import[plan_import]
    import[import]
  end

  subgraph backup["scry_core::backup"]
    bk_export[export]
    bk_plan_import[plan_import]
  end

  subgraph store["Store"]
    st_get_project_by_id[get_project_by_id]
    st_get_all_projects[get_all_projects]
    st_get_all_sections_by_project_id[get_all_sections_by_project_id]
    st_get_all_tasks_by_project_id[get_all_tasks_by_project_id]
    st_get_all_notes_by_project_id[get_all_notes_by_project_id]
    st_import[import]
  end

  h_import ==> import

  export -.-> bk_export
  plan_import -.-> bk_plan_import
  import --> st_import
  bk_export --> st_get_project_by_id
  bk_export --> st_get_all_projects
  bk_export --> st_get_all_sections_by_project_id
  bk_export --> st_get_all_tasks_by_project_id
  bk_export --> st_get_all_notes_by_project_id
  bk_plan_import --> st_get_all_projects

  classDef storeNode fill:#eef3fb,stroke:#5b7fae,stroke-width:1px
  class st_get_project_by_id,st_get_all_projects,st_get_all_sections_by_project_id,st_get_all_tasks_by_project_id,st_get_all_notes_by_project_id,st_import storeNode
```

## Coverage

- The six diagrams together touch all 30 `Store` trait methods (8 task, 8 note, 6 section, 7 project, 1 import) and every REST handler in `src/server/handlers.rs` except `index`, which only reports server metadata.
- `get_note_by_id`, `get_all_notes_by_task_id`, `update_note`, `delete_note` and `find_note_ids_by_handle` are marked `dead_code` on the trait and are reached only by the note scaffolding methods `get_note`, `list_notes_by_task_id`, `update_note`, `delete_note` and `get_note_by_handle`.

## Notes

- `export` and `plan_import` live in `scry_core::backup` and take `&dyn Store`, so the service reaches the store through them rather than calling it directly.
- Reads may use `Store` directly in principle, but in this codebase every store call goes through `ProjectService`.
