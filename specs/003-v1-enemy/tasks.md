# Tasks: Marco C — inimigo: peça e robô vermelho (v1 raw port)

**Input**: Design documents from `/specs/003-v1-enemy/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D15, §E), data-model.md, contracts/, quickstart.md (todos aprovados, commit `6cedcb4`)

**Fase**: v1 — Raw Port (Princípio I, constituição v1.3.0). Nenhuma task pode introduzir
abstração, refatoração, otimização, teste unitário ou infraestrutura. Se algo assim parecer
necessário, vira entrada em `docs/v2-backlog.md`, não task. **Nenhuma correção de bug** está
prevista: se um defeito objetivo do upstream surgir durante o port, PARAR e reportar — a cláusula
exige declaração na spec antes de qualquer commit; a peça recebeu uma correção declarada (FR-028–FR-032, Phase 3b): `docs/upstream-bugs.md` fica com 2 entradas.

**Tests**: não há testes automatizados nesta fase (plan.md "Testing"). A validação de cada story
é o ciclo do quickstart (build → import headless → cena headless → verificações mecânicas →
contrato → validação visual do usuário).

**Organization**: uma phase por user story, na ordem obrigatória US1 → US2 (spec FR-026;
`docs/port-order.md` itens 9 → 10). As stories **não** são paralelizáveis entre si: cada uma
termina com um commit próprio na `main` e a seguinte começa da árvore limpa; a US2 depende de
`Part` em Rust (acesso tipado a `explode`) e as duas editam o mesmo `red_robot.tscn` (a US1
desloca as linhas da US2).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência de task incompleta) — raro
  neste marco, porque build depende do módulo, cena depende do build, validação depende da cena.
- **[Story]**: US1, US2 (spec.md)
- Caminhos relativos à raiz do repositório (`oxide-godot/` = projeto Godot;
  `oxide_godot_core/oxide_godot_lib/src/` = crate Rust).

## Path Conventions

```
oxide_godot_core/oxide_godot_lib/src/lib.rs        ExtensionLibrary + `mod` de cada módulo (só isso)
oxide_godot_core/oxide_godot_lib/src/<módulo>.rs   uma classe por script (research.md §"Mapa por script")
oxide_godot_core/oxide_godot_lib/src/player.rs     Marco B — só visibilidade muda (US2)
oxide-godot/enemies/red_robot/red_robot.tscn       11.053 linhas; editada nas DUAS stories (plan.md "Edição das cenas")
oxide-godot/enemies/red_robot/parts/part.gd + .uid       apagados no commit da US1
oxide-godot/enemies/red_robot/red_robot.gd + .uid        apagados no commit da US2
docs/v2-backlog.md                                 uma linha por melhoria percebida (itens 15–18)
docs/upstream-bugs.md                              entrada #2 na Phase 3b (correção da peça)
../oxide_godot_origins/                            referência intocada do GDScript original
```

Decisões fechadas (valem para as duas stories — instrução, não opção; detalhes em research.md):

- **Part**: `#[export] #[var(set = set_fade_value)] fade_value: f32` com `#[func] fn set_fade_value`
  no bloco `#[godot_api] impl Part` principal; em `process`, **chamar `self.set_fade_value(..)`**,
  nunca atribuir ao campo (D2). `ready` duplica material da superfície 0 e o `next_pass` da cópia
  com `Gd::duplicate_resource()` (o `duplicate()` gerado está deprecado e gera warning), só se
  `!Os::singleton().has_feature("dedicated_server")` (D3). `#[func] pub(crate) fn explode`
  (`#[func]` porque `red_robot.gd` chama por nome até a US2; `pub(crate)` para o acesso tipado da
  US2) (D1, D4). Timers: `self.base().get_tree().create_timer(t).signals().timeout().connect_other(&*self, |this| ...)`
  — `create_timer` retorna `Gd` direto, **sem `unwrap`** (D4). `destroy` instancia
  `part_disappear.tscn` como `instantiate_as::<CpuParticles3D>` — **nunca** `PartDisappear` (D5).
- **RedRobot**: enum `State { Idle, Approach, Aim, Shooting }` com
  `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)]` (D7);
  `#[signal] fn exploded();` no bloco `#[godot_api] impl RedRobot` **único**, emissão
  `self.signals().exploded().emit()` (D6); RPCs `#[rpc(authority, call_local, unreliable)]` `hit` e
  `play_shoot`; `shoot`, `animate`, `_clip_ray` **privados sem `#[func]`**; `player: Option<Gd<Node3D>>`.
- **Transformação inversa** (`Vector3 * Transform3D`): `gt.basis.transposed() * (v - gt.origin)` —
  **nunca** `affine_inverse()` (D9, confirmado numericamente).
- **Raycasts**: 3 vezes **inline** (sem helper — D12); `PhysicsRayQueryParameters3D::create_ex(from, to).collision_mask(0xFFFFFFFF).exclude(&array![rid]).done()`
  com `let rid = self.base().get_rid()` — exclusão **efetiva**, NÃO `Rid::Invalid`; `intersect_ray`
  retorna `VarDictionary`; `col.collider == player` → comparar `instance_id()`.
- Eixo Y: `gt.basis.col_b()`; `atan2(a, b)` → `a.atan2(b)`; graus via `.to_degrees()`;
  `format!("parameters/hit{}/request", randi() % 3 + 1)` com valor `1.to_variant()` (D10–D11).
- **Blast**: `instantiate_as::<Node3D>` (nunca `Blast`), `add_child` na raiz da árvore,
  `set_global_position`; tremor 13,0 após timer 0,1 s com closure `move` capturando `Gd<Player>`
  e `player.clone().bind_mut().add_camera_shake_trauma(13.0)` (D13).
- `body.get_name() == StringName::from("Target")` (D13 — `"Target".into()` é ambíguo).
- **US2 abre visibilidade** em `player.rs`: `pub(crate) fn add_camera_shake_trauma` — só a palavra,
  task própria antes do build, `git diff --stat` = `1 +-`, citada na mensagem do commit (D1).
- Um commit por script na `main`, mensagem no formato do quickstart §8; fix após checkpoint =
  commit `Fix port …`.

---

## Phase 1: Setup

**Purpose**: registrar a baseline contra a qual cada port é comparado. Nada de infraestrutura
(Princípio I).

- [x] T001 Confirmar pré-condições: nenhum editor Godot aberto (`pgrep -a godot` vazio — se houver, avisar o usuário e aguardar, nunca matar); `git status --short` vazio na `main`; anotar `git rev-parse --short HEAD` (esperado `6cedcb4` ou posterior sem commits "Port …"). O commit-base `4bb8f7f` usado por `quickstart.md`/T029 para `git diff -- '*.gd'` e `git log 4bb8f7f..HEAD` continua válido (plan/tasks não tocam `.gd`); `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 7
- [x] T002 Registrar baseline do build: `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` → esperado `0`; anotar em `specs/003-v1-enemy/quickstart.md` §"Baseline" se diferir
- [x] T003 Registrar baseline do import: `cd oxide-godot && /usr/bin/godot.x86_64 --headless --import --path . 2>&1 | tee /tmp/import.log`; confirmar `grep -n 'Initialize godot-rust' /tmp/import.log` (linha 1) e `grep -nE 'ERROR|SCRIPT ERROR' /tmp/import.log` vazio (ou só os 3 erros do upstream do `CLAUDE.md`)
- [x] T004 [P] Confirmar o diretório de bindings geradas usado pelo research.md: `ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1` → esperado `.../godot-core-aea5c50e7fda9d57/out`; se o hash diferir, atualizar a linha 6 de `specs/003-v1-enemy/research.md`
- [x] T005 Medir a baseline das cenas do marco (quickstart §"Baseline"): `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn 2>&1 | tee /tmp/base_robot.log` e idem `level/level.tscn` → `/tmp/base_level.log`; esperado exit 124 em ambas, `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked'` vazio, WARNINGs = 1 (`HDR`) no robô e 2 (`HDR`, `Physics interpolation`) no level. Confirmar também `grep -c '^| 1 ' docs/upstream-bugs.md` = 1 e `grep -c '^| [0-9]' docs/v2-backlog.md` = 14

**Checkpoint**: baseline conhecida — 0 warnings, extensão carrega, 0 `ERROR` no import e nas duas cenas.

---

## Phase 2: Foundational

**Não existe neste marco.** Os dois ports são estritamente sequenciais (part → red_robot) e a
única edição compartilhada é a linha `mod <módulo>;` em `lib.rs`, feita dentro de cada story. A
abertura de visibilidade `pub(crate)` em `player.rs` **pertence à US2** (é o robô quem a exige e
ela entra no commit do robô); a de `Part::explode` nasce já `pub(crate)` na US1. Nenhum módulo
comum, helper, trait ou constante compartilhada pode ser criado (Princípio I) — os três raycasts
do robô ficam inline (research D12).

---

## Phase 3: User Story 1 — Peça do robô portada (Priority: P1) 🎯 MVP

**Goal**: `enemies/red_robot/parts/part.gd` (57 l.) → `Part: RigidBody3D`; sem cena própria — o
port é a troca de `type` nos 3 nodes `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead` de
`red_robot.tscn`. `fade_value` com setter no shader, replicado por nome; `explode()` exposto por
nome (o robô, ainda GDScript, chama `death_shield1.explode()`); fade → RPC `destroy` → puff.

**Independent Test**: `red_robot.tscn` e `level.tscn` headless sem `ERROR` (as 3 peças rodam
`ready` e duplicam materiais); contrato `contracts/part.md`; no jogo, matar um robô produz o mesmo
voo/queda/fade/puff das peças.

- [x] T006 [US1] Criar `oxide_godot_core/oxide_godot_lib/src/part.rs` traduzindo linha a linha `oxide-godot/enemies/red_robot/parts/part.gd`: `use godot::classes::{CollisionShape3D, CpuParticles3D, IRigidBody3D, Material, MeshInstance3D, MultiplayerSynchronizer, Node, Os, PackedScene, RigidBody3D, ShaderMaterial}; use godot::global::randf; use godot::prelude::*;`; `#[derive(GodotClass)] #[class(init, base=RigidBody3D)] pub struct Part { base: Base<RigidBody3D>, _mat: Option<Gd<Material>>, #[export] #[init(val = 3.0)] lifetime: f32, #[export] #[init(val = 3.0)] lifetime_random: f32, #[export] #[init(val = 0.5)] disappearing_time: f32, #[export] #[var(set = set_fade_value)] fade_value: f32, _disappearing_counter: f32 }`; `#[godot_api] impl IRigidBody3D for Part`: `ready` (`self.base_mut().set_process(false); if !Os::singleton().has_feature("dedicated_server") { let mesh_inst = self.base().get_node_as::<Node>("Model").get_child(0).unwrap().cast::<MeshInstance3D>(); let mut mesh = mesh_inst.get_mesh().unwrap(); let mut mat: Gd<Material> = mesh.surface_get_material(0).unwrap().duplicate_resource(); mesh.surface_set_material(0, &mat); let next_pass: Gd<Material> = mat.get_next_pass().unwrap().duplicate_resource(); mat.set_next_pass(&next_pass); self._mat = Some(mat); }`) e `process(&mut self, delta: f64)` (`let fade = (self._disappearing_counter / self.disappearing_time).powi(2); self.set_fade_value(fade); self._disappearing_counter += delta as f32; if self._disappearing_counter >= self.disappearing_time - 0.2 { self.base_mut().rpc("destroy", &[]); self.base_mut().set_process(false); }`); `#[godot_api] impl Part` (bloco único): `#[func] fn set_fade_value(&mut self, value: f32) { self.fade_value = value; if let Some(mat) = &self._mat { mat.get_next_pass().unwrap().cast::<ShaderMaterial>().set_shader_parameter("emission_cutout", &value.to_variant()); } }`; `#[func] pub(crate) fn explode(&mut self)` (`// Start synching.` + `self.base().get_node_as::<MultiplayerSynchronizer>("MultiplayerSynchronizer").set_visibility_public(true); self.base_mut().set_freeze_enabled(false); if !self.base().get_multiplayer().unwrap().is_server() { return; } self.base().get_node_as::<CollisionShape3D>("Col1").set_disabled(false); ...("Col2")...; self.base_mut().set_linear_velocity(3.0 * Vector3::UP); let angular = (Vector3::new(randf() as f32, randf() as f32, randf() as f32).normalized() * 2.0 - Vector3::ONE) * 10.0; self.base_mut().set_angular_velocity(angular); let wait = self.lifetime + self.lifetime_random * randf() as f32; self.base().get_tree().create_timer(wait as f64).signals().timeout().connect_other(&*self, |this: &mut Part| this.base_mut().set_process(true));`); `#[rpc(authority, call_local, unreliable)] fn destroy(&mut self)` (`let mut puff: Gd<CpuParticles3D> = load::<PackedScene>("res://enemies/red_robot/parts/part_disappear_effect/part_disappear.tscn").instantiate_as::<CpuParticles3D>(); self.base().get_parent().unwrap().add_child(&puff); let origin = self.base().get_global_transform().origin; puff.set_global_position(origin); self.base().get_tree().create_timer(0.2).signals().timeout().connect_other(&*self, |this: &mut Part| this.base_mut().queue_free());`). Nenhum outro `#[func]` (research D1–D5)
- [x] T007 [US1] Adicionar `mod part;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod door;`; nada mais muda em lib.rs)
- [x] T008 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `cargo build 2>&1 | grep -c '^warning'` = 0 (atenção: `duplicate()` gera warning de deprecação — usar `duplicate_resource()`). Em caso de erro, consultar research.md D2–D5 e as bindings em `oxide_godot_core/target/debug/build/godot-core-*/out/classes/` antes de improvisar; não alterar `Cargo.toml`
- [x] T009 [US1] Editar `oxide-godot/enemies/red_robot/red_robot.tscn` como texto (11.053 linhas — reconferir com `grep -n 'name="PartShield1"\|name="PartShield2"\|name="PartHead"\|^script = ExtResource("24")\|parts/part.gd' oxide-godot/enemies/red_robot/red_robot.tscn` → esperado l.10833/10885/10936, l.10841/10892/10944, l.26): nas 3 linhas de node trocar `type="RigidBody3D"` por `type="Part"`; remover as 3 linhas `script = ExtResource("24")`; remover a linha `[ext_resource type="Script" uid="uid://c3vo80hyj6w6c" path="res://enemies/red_robot/parts/part.gd" id="24"]` (l.26). Editar de baixo para cima (10944 → 10892 → 10841 → nodes → l.26) ou por padrão de texto, para os números não deslocarem durante a edição. **MANTER** em cada peça `transform`, `collision_layer = 3`, `collision_mask = 3`, `mass = 2000.0`, `physics_material_override`, `freeze = true`, `angular_damp = 0.3` e os filhos `MultiplayerSynchronizer` (`replication_config`, `public_visibility = false`), `Model`, `Col1`, `Col2`. Conferir antes que nenhuma das 4 exports (`lifetime`, `lifetime_random`, `disappearing_time`, `fade_value`) está gravada nos blocos (esperado: nenhuma). Diff esperado: `4 deletions, 3 changed lines` (`git diff --stat` = `7 +++----`)
- [x] T010 [US1] Apagar `oxide-godot/enemies/red_robot/parts/part.gd` e `oxide-godot/enemies/red_robot/parts/part.gd.uid` (`git rm`); verificar `grep -rn 'uid://c3vo80hyj6w6c' oxide-godot/ | grep -v '/.godot/'` vazio e `grep -rn 'parts/part.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [x] T011 [US1] Validação headless (quickstart §2–3; sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn 2>&1 | tee /tmp/run.log` e depois `level/level.tscn` → exit 124; `grep -nE 'ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked' /tmp/run.log` vazio em ambas (só os WARNINGs da baseline T005). As 3 peças instanciam a classe Rust: `ready` roda `get_node("Model").get_child(0)`, duplica material e `next_pass` — qualquer caminho errado aparece aqui
- [x] T012 [US1] Verificações mecânicas e contrato (quickstart §4–5; `contracts/part.md` §"Verificação antes do commit"): `grep -c 'type="Part"' oxide-godot/enemies/red_robot/red_robot.tscn` = 3; `grep -c 'ExtResource("24")' ...red_robot.tscn` = 0; `grep -c 'public_visibility = false' ...red_robot.tscn` = 3; `grep -n 'properties/0/path = NodePath(".:fade_value")' ...red_robot.tscn` = 1 linha (≈l.10418) — o nome existe em `part.rs` como `#[export] #[var(set)]`; `grep -n 'explode()' oxide-godot/enemies/red_robot/red_robot.gd` = l.96, 97, 98 (o robô GDScript segue chamando por nome — `explode` é `#[func]`); `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `parts/part.gd`; `ls oxide_godot_core/oxide_godot_lib/src/` = 10 arquivos (lib.rs + 9 módulos); `grep -n '#\[func\]\|#\[rpc' -A1 oxide_godot_core/oxide_godot_lib/src/part.rs | grep 'fn '` = `set_fade_value`, `explode`, `destroy`
- [x] T013 [US1] Registrar em `docs/v2-backlog.md` a linha nº 15 (research.md §"Backlog v2 candidato"): `enemies/red_robot/parts/part.gd` (port 1) — instanciar o puff no pai do robô (ou na raiz) em vez do pai da peça (`Death`); motivação: o puff nasce sob o robô, removido 10 s após a morte — os tempos hoje não se cruzam, mas a dependência é frágil. Não duplicar os itens 1–14
- [x] T014 [US1] Commit único do port 1 na `main` (autor the repository author), incluindo `src/part.rs`, `src/lib.rs`, `oxide-godot/enemies/red_robot/red_robot.tscn`, as deleções de `parts/part.gd`/`.uid` e `docs/v2-backlog.md`. Mensagem: `Port part.gd → Part (RigidBody3D); red_robot.tscn: nodes Death/PartShield1, Death/PartShield2, Death/PartHead type="RigidBody3D"→"Part"` + corpo: notas (`fade_value` com setter no shader, acionado também pela replicação; material e `next_pass` duplicados por `duplicate_resource()` fora de servidor dedicado; `explode` exposto por nome para o robô GDScript e `pub(crate)` para o port 2; `await` → timers com `connect_other`; puff instanciado como `CpuParticles3D` por API base; quirk preservado: puff no pai da peça); `- backlog v2: item 15`. Anotar o hash
- [x] T015 [US1] **Checkpoint do usuário (validação visual, plan.md "Validação visual" port 1)** — feito pelo usuário no jogo, comparando com `../oxide_godot_origins/`: matar um robô (5 tiros) → os dois escudos e a cabeça se soltam para cima com rotação aleatória, caem e quicam com física, ficam 3–6 s no chão, somem num fade (~0,3 s, brilho do `emission_cutout`) e terminam com o puff; cada peça some no seu próprio tempo, sem afetar as outras nem robôs vizinhos; console sem erro. O robô ainda é GDScript chamando `explode()` por nome. No editor, `red_robot.tscn`: os 3 nodes com tipo `Part`, sem script, `freeze` marcado, `MultiplayerSynchronizer` filho com `public_visibility` desmarcado. Divergência → commit `Fix port part.gd …`. Só seguir para a US2 com o OK explícito

**Checkpoint**: 6 `.gd` restantes; `red_robot.tscn` com 3 nodes `Part`; jogo jogável.

---

## Phase 3b: User Story 1 — correção conservadora do bug do upstream na peça (FR-028–FR-032)

**Goal**: emenda descoberta na revisão do port 1 (harness de paridade): o material duplicado era instalado no `Mesh` compartilhado pelos dois escudos; corrigir com override por instância. Commit próprio `Fix port part.gd …`; `9aee2b8` não é reescrito.

- [x] T032 [US1] Editar `oxide_godot_core/oxide_godot_lib/src/part.rs`, só em `ready()`: substituir a linha `mesh.surface_set_material(0, &mat);` por `mesh_inst.set_surface_override_material(0, &mat);` (`MeshInstance3D::set_surface_override_material(surface: i32, material: impl AsArg<Option<Gd<Material>>>)`, bindings `mesh_instance_3d.rs`; `mesh_inst` precisa ser `mut`), com as duas linhas de comentário **imediatamente acima**: `// upstream bug fix: part.gd instalava a cópia do material no recurso Mesh compartilhado pelos dois escudos` / `// (surface_set_material), então o último escudo a entrar "vencia" e o fade do outro nunca era renderizado; override por instância.` A leitura `mesh.surface_get_material(0)` (origem da cópia), a duplicação do `next_pass`, `self._mat = Some(mat)` e todo o resto do arquivo ficam iguais. Se `mesh` deixar de precisar de `mut`, remover o `mut` (0 warnings)
- [x] T033 [US1] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → 0 warnings
- [x] T034 [US1] Validação headless: import (`Initialize godot-rust`, sem `ERROR` novo); `timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn` e `level/level.tscn` → grep de regressão vazio
- [x] T035 [US1] Verificação do resultado (FR-032), com script `SceneTree` descartável fora do repo (não commitar): instanciar `red_robot.tscn`, pegar `Death/PartShield1`, `Death/PartShield2`, `Death/PartHead`; para cada, `mi = get_node("Model").get_child(0)`: `mi.get_surface_override_material(0)` não nulo e distinto entre as 3 peças; `mi.mesh.surface_get_material(0)` igual entre os dois escudos (mesh compartilhado intocado) e sem `next_pass` duplicado por elas; `set_indexed("fade_value", 0.7)` em `PartShield1` muda `get_surface_override_material(0).next_pass.get_shader_parameter("emission_cutout")` de `PartShield1` para 0.7 e o de `PartShield2` continua 0.0. Reportar as saídas
- [x] T036 [US1] Verificações mecânicas: `git diff --stat` = só `part.rs` (≈ 3 +, 1 −) e `docs/upstream-bugs.md`; `grep -rn 'upstream bug fix' oxide_godot_core/oxide_godot_lib/src/` = 2 arquivos (door.rs, part.rs); `grep -n 'surface_set_material' part.rs` vazio; nenhum outro arquivo tocado
- [x] T037 [US1] Registrar em `docs/upstream-bugs.md` a entrada `2`: defeito "material dos escudos compartilhado — `part.gd:23-26` instalava a cópia com `mesh.surface_set_material` no `Mesh` compartilhado por `PartShield1`/`PartShield2`; o último escudo vencia, o fade do outro não era renderizado (verificado em headless no original)"; script/cena `enemies/red_robot/parts/part.gd:23-26` / `red_robot.tscn` (`Death/PartShield1`, `Death/PartShield2`, modelo ext_resource id="12"); spec `specs/003-v1-enemy` FR-028–FR-032; correção `src/part.rs ready()`: `set_surface_override_material(0, cópia)` no `MeshInstance3D` com comentário `// upstream bug fix`; commit = assunto do commit de T038
- [x] T038 [US1] Commit único na `main` (autor the repository author): `part.rs` + `docs/upstream-bugs.md`. Assunto: `Fix port part.gd → Part: surface override material per instance (upstream bug fix)`; corpo com a linha `- upstream bug fix: part.gd instalava a cópia do material no Mesh compartilhado pelos escudos (surface_set_material); agora override por instância (set_surface_override_material). Correção mínima; cena e modelos intocados; declarada na spec (FR-028–FR-032).` e `- docs/upstream-bugs.md: entrada #2.` Prefixo `Fix port` (não conta como commit `Port`)
- [x] T039 [US1] **Checkpoint do usuário**: no jogo, matar um robô e observar os **dois escudos** esmaecendo cada um no seu tempo (antes, um sumia sem fade); cabeça como antes. Só avançar para a US2 após OK

**Checkpoint**: `docs/upstream-bugs.md` com 2 entradas; `part.rs` com o único `upstream bug fix` do marco.

---

## Phase 4: User Story 2 — Robô vermelho portado (Priority: P2)

**Goal**: `enemies/red_robot/red_robot.gd` (283 l.) → `RedRobot: CharacterBody3D`; sinal
`exploded` (consumido por `level.gd:99`), RPCs `hit`/`play_shoot`, method tracks
`shoot_check`/`resume_approach`, handlers `_on_area_body_entered/_exited`; máquina de estados
IDLE→APPROACH→AIM→SHOOTING; acesso tipado a `Part` (`explode`) e `Player` (`add_camera_shake_trauma`);
`Blast` por API base.

**Independent Test**: `red_robot.tscn` (robô IDLE + gravidade) e `level.tscn` (spawn por
`level.gd`, `exploded` conectado por nome) headless sem `ERROR`; contrato `contracts/red-robot.md`;
no jogo, o ciclo detectar → aproximar → mirar → atirar → ser atingido → morrer → respawn
indistinguível do original.

- [ ] T016 [US2] Alterar **só a visibilidade** em `oxide_godot_core/oxide_godot_lib/src/player.rs`: `fn add_camera_shake_trauma(&mut self, amount: f64)` (dentro do `#[godot_api] impl Player`, atributo `#[rpc(...)]` fica onde está) → `pub(crate) fn add_camera_shake_trauma(&mut self, amount: f64)`. Conferir: `git diff --stat` = `player.rs | 2 +-` (1 par −/+, nada mais) (research D1)
- [ ] T017 [US2] Criar `oxide_godot_core/oxide_godot_lib/src/red_robot.rs` — parte 1 (declarações), traduzindo `oxide-godot/enemies/red_robot/red_robot.gd:1-54`: `use godot::classes::{AnimationPlayer, AnimationTree, AudioStreamPlayer3D, BoneAttachment3D, CharacterBody3D, CollisionShape3D, CpuParticles3D, ICharacterBody3D, MeshInstance3D, Node3D, Os, PackedScene, PhysicsRayQueryParameters3D, RayCast3D, ShaderMaterial}; use godot::global::randi; use godot::prelude::*; use crate::part::Part; use crate::player::Player;`; `#[derive(GodotConvert, Var, Export, Clone, Copy, PartialEq, Debug)] #[godot(via = i64)] pub enum State { Idle, Approach, Aim, Shooting }`; consts `f32`: `PLAYER_AIM_TOLERANCE_DEGREES = 15.0_f32.to_radians()`, `SHOOT_WAIT = 6.0`, `AIM_TIME = 1.0`, `AIM_PREPARE_TIME = 0.5`, `BLEND_AIM_SPEED = 0.05`; `#[derive(GodotClass)] #[class(init, base=CharacterBody3D)] pub struct RedRobot { base: Base<CharacterBody3D>, #[export] test_shoot: bool, #[export] target_position: Vector3, #[export] #[init(val = 5)] health: i32, #[export] #[init(val = State::Idle)] state: State, #[export] dead: bool, #[export] #[init(val = AIM_PREPARE_TIME)] aim_preparing: f32, #[init(val = SHOOT_WAIT)] shoot_countdown: f32, #[init(val = AIM_TIME)] aim_countdown: f32, player: Option<Gd<Node3D>>, orientation: Transform3D,` + 15 `OnReady` (research D8, caminhos completos): `animation_tree: AnimationTree` ("AnimationTree"), `shoot_animation: AnimationPlayer` ("ShootAnimation"), `model: Node3D` ("RedRobotModel"), `ray_from: BoneAttachment3D` ("RedRobotModel/Armature/Skeleton3D/RayFrom"), `ray_mesh: MeshInstance3D` (".../RayFrom/RayMesh"), `laser_raycast: RayCast3D` (".../RayFrom/RayCast"), `collision_shape: CollisionShape3D` ("CollisionShape3D"), `explosion_sound`/`hit_sound: AudioStreamPlayer3D` ("SoundEffects/Explosion", "SoundEffects/Hit"), `death: Node3D` ("Death"), `death_shield1`/`death_shield2`/`death_head: Part` ("Death/PartShield1", "Death/PartShield2", "Death/PartHead"), `death_detach_spark1`/`2: CpuParticles3D` ("Death/DetachSpark1", "Death/DetachSpark2") `}`. `blast_scene` (preload) não vira campo: `load` no ponto de uso em `shoot` (D13). Manter os comentários do original onde existem
- [ ] T018 [US2] `red_robot.rs` — parte 2 (virtuais e privados), traduzindo `red_robot.gd:57-69,108-168,171-256,268-271`: `#[godot_api] impl ICharacterBody3D for RedRobot` com `ready` (`self.orientation = self.base().get_global_transform(); self.orientation.origin = Vector3::ZERO; self.animation_tree.set_active(true); if self.test_shoot { self.shoot_countdown = 0.0; } if self.dead { self.model.set_visible(false); self.collision_shape.set_disabled(true); self.animation_tree.set_active(false); } self.animate(0.0);`) e `physics_process(&mut self, delta: f64)`: `if self.dead { return; }`; `if !is_server { self.animate(delta); return; }`; `if self.test_shoot { self.shoot(); self.test_shoot = false; }`; `let Some(player) = self.player.clone() else { self.target_position = Vector3::ZERO; self.animate(delta); let g = self.base().get_gravity() * delta as f32; self.base_mut().set_velocity(g); self.base_mut().set_up_direction(Vector3::UP); self.base_mut().move_and_slide(); return; };`; `self.target_position = player.get_global_transform().origin;`; APPROACH: decremento de `aim_preparing` até 0; `let gt = self.base().get_global_transform(); let to_player_local = gt.basis.transposed() * (self.target_position - gt.origin); let angle_to_player = to_player_local.x.atan2(to_player_local.z);` se `> -TOL && < TOL`: `shoot_countdown -= delta as f32; if < 0.0 {` raycast **inline** (D12): `let rid = self.base().get_rid(); let params = PhysicsRayQueryParameters3D::create_ex(ray_origin, player.get_global_transform().origin + Vector3::UP).collision_mask(0xFFFFFFFF).exclude(&array![rid]).done(); let col: VarDictionary = self.base().get_world_3d().unwrap().get_direct_space_state().unwrap().intersect_ray(&params.unwrap());` e `!col.is_empty() && col.get("collider").and_then(|v| v.try_to::<Gd<Object>>().ok()).map(|c| c.instance_id() == player.instance_id()).unwrap_or(false)` → `state = Aim; aim_countdown = AIM_TIME; aim_preparing = 0.0` senão `shoot_countdown = SHOOT_WAIT }`; AIM/SHOOTING: `max_dist = 1000.0; if self.laser_raycast.is_colliding() { max_dist = (self.ray_from.get_global_transform().origin - self.laser_raycast.get_collision_point()).length(); } self._clip_ray(max_dist);` `aim_preparing` sobe até `AIM_PREPARE_TIME`; `aim_countdown -= delta as f32; if < 0.0 && state == Aim {` mesmo raycast inline para `self.target_position + Vector3::UP`; acerta → `state = Shooting; shoot_countdown = SHOOT_WAIT; self.base_mut().rpc("play_shoot", &[]);` senão `self.resume_approach(); }`; depois `self.animate(delta); self.orientation = self.orientation * Transform3D::new(Basis::from_quaternion(self.animation_tree.get_root_motion_rotation()), self.animation_tree.get_root_motion_position());` velocidade/gravidade/`set_up_direction(UP)`/`move_and_slide` como no Player; `orientation.origin = ZERO; orientation = orientation.orthonormalized(); let basis = self.orientation.basis; self.base_mut().set_global_basis(basis);`. Bloco `impl RedRobot` **sem** `#[godot_api]` com: `fn shoot(&mut self)` (`let gt = self.ray_from.get_global_transform(); let ray_origin = gt.origin; let ray_dir = gt.basis.col_b(); let mut max_dist: f32 = 1000.0;` raycast inline de `ray_origin` a `ray_origin + ray_dir * max_dist`; `if !col.is_empty() { let position = col.get("position").unwrap().to::<Vector3>(); max_dist = ray_origin.distance_to(position); }` (o `if col.collider == player: pass # Kill.` não gera código); `self._clip_ray(max_dist); let mesh_offset = self.ray_mesh.get_position().z; let mut laser_ember = self.base().get_node_as::<CpuParticles3D>("RedRobotModel/Armature/Skeleton3D/RayFrom/LaserEmber"); laser_ember.set_position(Vector3::new(0.0, 0.0, -max_dist / 2.0 - mesh_offset)); let mut e = laser_ember.get_emission_box_extents(); e.z = (max_dist - mesh_offset.abs()) / 2.0; laser_ember.set_emission_box_extents(e); if !col.is_empty() { let position = ...; let mut blast: Gd<Node3D> = load::<PackedScene>("res://enemies/red_robot/laser/impact_effect/impact_effect.tscn").instantiate_as::<Node3D>(); self.base().get_tree().get_root().unwrap().add_child(&blast); blast.set_global_position(position); if let Some(player) = self.player.clone() { if <collider == player por instance_id> { if let Ok(player) = player.try_cast::<Player>() { self.base().get_tree().create_timer(0.1).signals().timeout().connect_other(&*self, move |_this: &mut RedRobot| { player.clone().bind_mut().add_camera_shake_trauma(13.0); }); } } } }`); `fn animate(&mut self, delta: f64)` (APPROACH: `to_player_local` pela transposta, `angle_to_player`, `set("parameters/state/transition_request", &"turn_left"/"turn_right"/"idle"/"walk".to_variant())` nas 4 condições na ordem do original; fora de APPROACH `"idle"`; `if self.target_position != Vector3::ZERO { set("parameters/aiming/blend_amount", &(self.aim_preparing / AIM_PREPARE_TIME).clamp(0.0, 1.0).to_variant()); let mt = self.ray_mesh.get_global_transform(); let to_cannon_local = mt.basis.transposed() * (self.target_position + Vector3::UP - mt.origin); let h_angle = to_cannon_local.x.atan2(-to_cannon_local.z).to_degrees(); let v_angle = to_cannon_local.y.atan2(-to_cannon_local.z).to_degrees(); let mut blend_pos = self.animation_tree.get("parameters/aim/blend_position").to::<Vector2>(); blend_pos.x += BLEND_AIM_SPEED * delta as f32 * -h_angle; blend_pos.x = blend_pos.x.clamp(-1.0, 1.0); blend_pos.y += BLEND_AIM_SPEED * delta as f32 * v_angle; blend_pos.y = blend_pos.y.clamp(-1.0, 1.0); set("parameters/aim/blend_position", &blend_pos.to_variant()); }`); `fn _clip_ray(&mut self, length: f32)` (`let mesh_offset = self.ray_mesh.get_position().z; if !Os::singleton().has_feature("dedicated_server") { self.ray_mesh.get_surface_override_material(0).unwrap().cast::<ShaderMaterial>().set_shader_parameter("clip", &(length + mesh_offset).to_variant()); }`) (research D9–D14)
- [ ] T019 [US2] `red_robot.rs` — parte 3 (bloco Godot), traduzindo `red_robot.gd:4,72-105,259-265,274-283`: `#[godot_api] impl RedRobot` **único** com `#[signal] fn exploded();`; `#[func] fn resume_approach(&mut self) { self.state = State::Approach; self.aim_preparing = AIM_PREPARE_TIME; self.shoot_countdown = SHOOT_WAIT; }`; `#[rpc(authority, call_local, unreliable)] fn hit(&mut self)` (`if self.dead { return; } let param = format!("parameters/hit{}/request", randi() % 3 + 1); self.animation_tree.set(&param, &1.to_variant()); self.hit_sound.play(); self.health -= 1; if self.health == 0 { self.dead = true; self.animation_tree.set_active(false); self.model.set_visible(false); self.death.set_visible(true); self.collision_shape.set_disabled(true); self.death_detach_spark1.set_emitting(true); self.death_detach_spark2.set_emitting(true); self.death_shield1.bind_mut().explode(); self.death_shield2.bind_mut().explode(); self.death_head.bind_mut().explode(); self.explosion_sound.play(); self.signals().exploded().emit(); if self.base().get_multiplayer().unwrap().is_server() { self.base().get_tree().create_timer(10.0).signals().timeout().connect_other(&*self, |this: &mut RedRobot| this.base_mut().queue_free()); } }`); `#[rpc(authority, call_local, unreliable)] fn play_shoot(&mut self) { self.shoot_animation.play_ex().name("shoot").done(); }`; `#[func] fn shoot_check(&mut self) { self.test_shoot = true; }`; `#[func] fn _on_area_body_entered(&mut self, body: Gd<Node3D>) { if body.clone().try_cast::<Player>().is_ok() || body.get_name() == StringName::from("Target") { self.player = Some(body); self.state = State::Approach; } }`; `#[func] fn _on_area_body_exited(&mut self, body: Gd<Node3D>) { if body.try_cast::<Player>().is_ok() { self.player = None; self.state = State::Idle; } }`. Nenhum outro `#[func]`/`#[signal]`/`#[rpc]` (research D6, D14)
- [ ] T020 [US2] Adicionar `mod red_robot;` em `oxide_godot_core/oxide_godot_lib/src/lib.rs` (após `mod part;`)
- [ ] T021 [US2] Build: `cd oxide_godot_core && cargo build 2>&1 | tail -20` → `Finished`; `grep -c '^warning'` = 0. Em caso de erro, research.md D6–D14 e as bindings antes de improvisar
- [ ] T022 [US2] Editar `oxide-godot/enemies/red_robot/red_robot.tscn` como texto (reconferir com `grep -n 'name="RedRobot" type=\|^script = ExtResource("1")\|red_robot.gd' oxide-godot/enemies/red_robot/red_robot.tscn` → esperado ≈l.10583 (raiz) e ≈l.10586 (`script`), l.3 — a raiz deslocou só −1 após a US1: o `ext_resource` l.26; as 3 linhas `script` das peças ficam abaixo dela): na linha da raiz trocar `type="CharacterBody3D"` por `type="RedRobot"`; remover `script = ExtResource("1")`; remover a linha `[ext_resource type="Script" uid="uid://bf14mo0lrrvjl" path="res://enemies/red_robot/red_robot.gd" id="1"]` (l.3). **MANTER** `collision_layer = 3`/`collision_mask = 3` da raiz, `MultiplayerSynchronizer` com `replication_config`, `AnimationTree`, `ShootAnimation` (method tracks `shoot_check`/`resume_approach`), `PlayerDetectionArea`, os 3 nodes `Part`, e as 2 `[connection …]` do fim do arquivo. Diff esperado: `4 +---` (1 troca + 2 remoções)
- [ ] T023 [US2] Apagar `oxide-godot/enemies/red_robot/red_robot.gd` e `oxide-godot/enemies/red_robot/red_robot.gd.uid` (`git rm`); verificar `grep -rn 'uid://bf14mo0lrrvjl' oxide-godot/ | grep -v '/.godot/'` vazio e `grep -rn 'red_robot.gd' oxide-godot/ --include='*.tscn' --include='*.gd'` vazio
- [ ] T024 [US2] Validação headless (sem editor aberto): import com `Initialize godot-rust` e sem `ERROR` novo; `cd oxide-godot && timeout 20 /usr/bin/godot.x86_64 --headless --path . enemies/red_robot/red_robot.tscn 2>&1 | tee /tmp/run.log` → exit 124, grep de regressão vazio (robô isolado: 15 `OnReady` resolvem, `ready` ativa o `AnimationTree` e chama `animate(0)`, `physics_process` roda o ramo sem jogador — IDLE + gravidade); `level/level.tscn` → exit 124, grep vazio (`level.gd:97-100` instancia, atribui `transform`, conecta `exploded` **por nome**, `add_child(robot, true)`; se a área de detecção alcançar o Player spawnado, APPROACH/AIM rodam também). Só os WARNINGs da baseline
- [ ] T025 [US2] Verificações mecânicas e contrato (`contracts/red-robot.md` §"Verificação antes do commit"): `grep -c 'type="RedRobot"' oxide-godot/enemies/red_robot/red_robot.tscn` = 1; `grep -c 'ExtResource("1")' ...` = 0; `grep -n '"method": &"shoot_check"\|"method": &"resume_approach"' ...` = 2 linhas; `grep -n 'method="_on_area_body_entered"\|method="_on_area_body_exited"' ...` = 2 linhas; `grep -n 'properties/[0-9]/path' ... | head -5` = `.:global_transform`, `.:health`, `.:state`, `.:target_position`, `.:dead` — os 4 do script existem em `red_robot.rs` como `#[export]`; `grep -n 'exploded\|RedRobot' oxide-godot/level/level.gd` = l.6, 97, 99; `grep -n 'has_method("hit")' oxide_godot_core/oxide_godot_lib/src/bullet.rs` = 1; `git diff --stat HEAD -- 'oxide-godot/**/*.gd'` mostra só a deleção de `red_robot.gd`; `git diff HEAD -- oxide_godot_core/oxide_godot_lib/src/player.rs | grep '^[-+]' | grep -v '^[-+][-+]'` = exatamente 2 linhas (`-    fn add_camera_shake_trauma(` / `+    pub(crate) fn add_camera_shake_trauma(`); `git diff --stat HEAD -- oxide_godot_core/oxide_godot_lib/src/part.rs` vazio; `ls src/` = 11 arquivos; `grep -n '#\[func\]\|#\[rpc\|#\[signal\]' -A1 oxide_godot_core/oxide_godot_lib/src/red_robot.rs | grep 'fn '` = `exploded`, `resume_approach`, `hit`, `play_shoot`, `shoot_check`, `_on_area_body_entered`, `_on_area_body_exited` (7, nenhum outro)
- [ ] T026 [US2] Registrar em `docs/v2-backlog.md` as linhas nº 16, 17 e 18 (research.md §"Backlog v2 candidato"): 16 — `enemies/red_robot/red_robot.gd` (port 2): remover o ramo morto `body.name == "Target"` e tipar `player` como `Gd<Player>` (nenhuma cena tem node `Target`; a referência genérica obriga `try_cast` em cada uso); 17 — substituir o `await` de 10 s dentro do RPC `hit` por timer/sinal fora do RPC (remoção acoplada ao handler de dano); 18 — replicar `aim_preparing` (ou não exportá-lo) e tirar `test_shoot` do inspector (exportados fora da `SceneReplicationConfig`; `test_shoot` é gatilho interno do method track). Não duplicar os itens 1–15
- [ ] T027 [US2] Commit único do port 2 na `main` (autor the repository author), incluindo `src/red_robot.rs`, `src/lib.rs`, `src/player.rs`, `oxide-godot/enemies/red_robot/red_robot.tscn`, as deleções de `red_robot.gd`/`.uid` e `docs/v2-backlog.md`. Mensagem: `Port red_robot.gd → RedRobot (CharacterBody3D); red_robot.tscn: node RedRobot type="CharacterBody3D"→"RedRobot"` + corpo: notas (sinal `exploded` registrado no bloco principal e conectado por nome pelo `level.gd`; enum `State` `via = i64`; `Vector3 * Transform3D` traduzido como `basis.transposed() * (v - origin)`; raycasts com exclusão efetiva pelo RID do robô e `collider == player` por `instance_id`; acesso tipado a `Part::explode` e `Player::add_camera_shake_trauma` (após `try_cast`, com atraso de 0,1 s); `Blast` por API base; `await`s → timers com `connect_other`; quirks preservados: `body.name == "Target"`, `pass # Kill.`, `player: Node3D`, 10 s dentro do `hit`, `aim_preparing`/`test_shoot` não replicados); `- player.rs: só visibilidade pub(crate) em add_camera_shake_trauma (acesso tipado do robô, FR-018); nenhuma lógica movida`; `- backlog v2: itens 16, 17, 18`. Anotar o hash
- [ ] T028 [US2] **Checkpoint do usuário (validação visual, plan.md port 2)** — feito pelo usuário no jogo, comparando com `../oxide_godot_origins/`: robô parado ao longe (IDLE); ao aproximar, vira (`turn_left/right`) e anda até ficar de frente; ~6 s de frente → mira (laser vermelho clipado no cenário/jogador, `LaserEmber` ao longo do raio, animação de mira acompanha); ~1 s → atira (animação "shoot", impacto no ponto, **tremor forte** se acertar) e volta a aproximar; cada tiro recebido → uma das 3 animações de dano + som; 5º tiro → morte (peças, faíscas, som de explosão); robô some 10 s depois; 15 s após a morte outro robô nasce no mesmo ponto (level, sinal `exploded`); sair da área de detecção → volta a IDLE. No editor, `red_robot.tscn`: raiz `RedRobot`, sem script, `MultiplayerSynchronizer` com `replication_config`, conexões da `PlayerDetectionArea` no painel de sinais. Divergência → commit `Fix port red_robot.gd …`. Só seguir para o Polish com o OK explícito

**Checkpoint**: 5 `.gd` restantes; `red_robot.tscn` inteiro em Rust; jogo jogável.

---

## Phase 5: Polish — verificação final do marco

**Purpose**: apenas a verificação final do quickstart e a validação visual completa. Sem
documentação extra, sem README, sem refatoração.

- [ ] T029 Verificação mecânica do marco (quickstart §"Verificação final"): `find oxide-godot -name '*.gd' -not -path '*/addons/*' | wc -l` = 5; `find oxide-godot -name '*.gd.uid' -not -path '*/addons/*' | wc -l` = 5; `git diff --stat 4bb8f7f -- 'oxide-godot/**/*.gd'` mostra exatamente 2 deleções (`parts/part.gd`, `red_robot.gd`) e nenhuma modificação (os 5 restantes byte a byte iguais — SC-001); `git log --oneline 4bb8f7f..HEAD | grep -c '^[0-9a-f]* Port '` = 2 (`Fix port …` não contam); `ls oxide_godot_core/oxide_godot_lib/src/` = `lib.rs` + 10 módulos (`debug_label part_disappear blast camera_noise_shake player_input player bullet door part red_robot`); `grep -c '^| 1 ' docs/upstream-bugs.md` = 1 e `git diff --stat 4bb8f7f -- docs/upstream-bugs.md CLAUDE.md` vazio; `grep -c '^| [0-9]' docs/v2-backlog.md` = 18; FR-018: `grep -nE '\.call\(|\.call_deferred\(|get_script' oxide_godot_core/oxide_godot_lib/src/part.rs oxide_godot_core/oxide_godot_lib/src/red_robot.rs` vazio; `grep -nE '\.get\("' oxide_godot_core/oxide_godot_lib/src/red_robot.rs` → só `"parameters/aim/blend_position"`, `"position"`, `"collider"`; `grep -nE '\.set\(' oxide_godot_core/oxide_godot_lib/src/red_robot.rs` → só `"parameters/…"` e o `set(&param, …)` do `hit`; `grep -n 'affine_inverse\|Rid::Invalid' oxide_godot_core/oxide_godot_lib/src/red_robot.rs` vazio
- [ ] T030 Validação headless final (sem editor aberto): `cd oxide_godot_core && cargo build 2>&1 | grep -c '^warning'` = 0; import headless com `Initialize godot-rust` e sem `ERROR`; `enemies/red_robot/red_robot.tscn`, `level/level.tscn`, `player/player.tscn`, `player/bullet/bullet.tscn`, `door/door.tscn` headless com grep de regressão vazio
- [ ] T031 **Validação visual completa do usuário (SC-002)**: menu → level; robôs patrulham/viram, miram com laser clipado, atiram (impacto + tremor 13,0 ao acertar), reagem a tiros (animação + som), morrem no 5º tiro (peças voam, faíscas, som, fade + puff entre 3 e 6,5 s), respawn 15 s depois; jogador (Marco B) e efeitos (Marco A) continuam iguais — tudo indistinguível de `../oxide_godot_origins/` em sessão lado a lado; `red_robot.tscn` no editor com raiz `RedRobot` e 3 nodes `Part`, sem scripts. Marco C concluído com o OK do usuário; então marcar T001–T031 `[x]` e commitar só `tasks.md` (`Tasks 003: marco C concluído`)

---

## Dependencies & Execution Order

### Ordem obrigatória

```
Phase 1 (Setup: T001–T005)          — baseline de red_robot.tscn e level.tscn
  → Phase 3 US1 (T006–T015)  → commit 1  (Part; red_robot.tscn: 3 nodes + ext_resource id="24")
  → Phase 4 US2 (T016–T028)  → commit 2  (RedRobot; red_robot.tscn: raiz + ext_resource id="1"; player.rs pub(crate))
  → Phase 5 Polish (T029–T031)
```

- **Phase 2 (Foundational)**: não existe — nada bloqueia as stories além do Setup.
- **Stories não são paralelizáveis entre si**: cada uma termina com um commit na `main` e a
  próxima parte da árvore limpa (FR-026, SC-004). US2 exige `Part` em Rust (`OnReady<Gd<Part>>`,
  `bind_mut().explode()`), e as duas editam `red_robot.tscn` — a US1 desloca as linhas da US2
  em −1 (só o `ext_resource` l.26; as 3 linhas `script` das peças, l.10841+, ficam abaixo da raiz e não a deslocam — elas deslocam −4 apenas as linhas ≥ 10841, como as conexões).
- **Ordem interna de cada story** (dependências estritas): [US2: visibilidade `pub(crate)` em
  `player.rs`] → módulo `.rs` → `mod` em `lib.rs` → `cargo build` → editar `.tscn` → apagar
  `.gd`/`.uid` → headless → verificações mecânicas + contrato → backlog → commit → checkpoint do
  usuário. A `.tscn` só é editada depois do build porque a classe precisa existir na lib
  carregada para o `type` resolver no import headless.
- **Checkpoint do usuário** (T015, T028, T031) é bloqueante: a story seguinte só começa com o
  OK explícito.

### Parallel Opportunities

Praticamente nenhuma, por construção:

- Setup: T004 é [P] em relação a T002/T003/T005 (só lê o diretório de bindings).
- Dentro de cada story, nenhuma task é [P]: cada passo consome o resultado do anterior.
  T017→T018→T019 escrevem o mesmo `red_robot.rs` em sequência.
- Backlog (T013, T026) poderia ser escrito a qualquer momento antes do commit da story, mas
  edita `docs/v2-backlog.md`, compartilhado — manter sequencial.

### Parallel Example

```bash
# Único par realmente independente (Phase 1):
Task: "T002 cargo build → contar warnings"
Task: "T004 ls -dt oxide_godot_core/target/debug/build/godot-core-*/out | head -1"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1: Setup (T001–T005) — baseline registrada.
2. Phase 3: US1 (T006–T015) — `part.gd` portado, commit 1, OK do usuário.
3. **PARAR E VALIDAR**: primeira classe sem cena própria (troca de `type` em 3 nodes da mesma
   cena), primeiro setter que escreve num shader e primeiro `duplicate_resource()`; qualquer
   problema de caminho de node (`Model` → filho 0), material ou replicação aparece aqui.

### Incremental Delivery

Cada story é um port completo e o jogo fica jogável após cada commit:

1. US1 → 6 `.gd` restantes → jogável (robô GDScript chama `explode()` por nome na classe Rust)
2. US2 → 5 → jogável (inimigo inteiro em Rust; `level.gd` recebe `exploded` por nome)
3. Polish → marco C concluído

### Se algo falhar no meio de uma story

Não commitar parcial. Ou o port inteiro (módulo + cena + deleção [+ visibilidade]) entra no
commit, ou nada: `git checkout -- oxide-godot/ oxide_godot_core/ docs/ && git clean -f oxide_godot_core/oxide_godot_lib/src/<módulo>.rs`
volta à árvore limpa da story anterior, que é sempre jogável. Se a implementação exigir algo não
previsto nas tasks (outro arquivo, outra visibilidade, uma correção), parar e reportar.

---

## Notes

- Nenhuma task cria helper, trait, módulo comum, teste ou log novo. Se parecer necessário, é
  entrada em `docs/v2-backlog.md` — não task. Os três raycasts do robô ficam inline (D12).
- **Nenhuma correção de bug** neste marco. Qualquer defeito objetivo encontrado → parar e
  reportar (a cláusula exige spec primeiro); qualquer outra coisa é melhoria → backlog. Em caso
  de dúvida, é melhoria.
- `red_robot.tscn` (11.053 linhas) é editada como texto nas duas stories; reconferir linhas com
  `grep -n` imediatamente antes de cada `sed` (as linhas citadas são as de 2026-09-15; a US1
  desloca as da US2).
- Nomes de `#[func]`, RPCs, sinal e propriedades `#[export]`/`#[var]` são contrato (FR-023) —
  copiar do GDScript, nunca "traduzir" (`_on_area_body_entered`, `shoot_check`, `fade_value`,
  `exploded`).
- O grep de regressão headless é `ERROR|SCRIPT ERROR|Invalid call|Invalid get|Invalid set|Nonexistent|panicked`;
  os WARNINGs da baseline (`HDR output`, `Physics interpolation`) não contam.
- Nenhum editor Godot aberto durante validações headless — avisar o usuário antes; nunca matar o
  processo dele.
- O commit (T014, T027) vem **antes** do checkpoint visual; divergência encontrada pelo usuário
  é corrigida em commit `Fix port …` na mesma story (nunca `Port …`, para
  `git log | grep -c '^[0-9a-f]* Port '` continuar = 2).
- Arquivos que **nunca** mudam neste marco: `Cargo.toml`, `.gdextension`, `project.godot`,
  `CLAUDE.md`, `docs/upstream-bugs.md` (exceto a entrada #2 da Phase 3b), os 5 `.gd` restantes, `specs/` (exceto o `[x]` em
  `tasks.md` ao final) e, em `player.rs`, qualquer coisa além da palavra `pub(crate)` de T016.
