# Pendências do BD Launcher

O que falta para o lançamento, e quem resolve. Nada aqui trava o desenvolvimento:
o código compila e os testes passam com os pins desligados.

## 1. Certificados de assinatura (BD)

Sem certificados próprios, o launcher aceita qualquer assinatura válida — e os pins
antigos (dos donos anteriores) foram removidos em `a9b4387` justamente para não
confiar na identidade deles. Quando os certificados da BD chegarem:

- **Windows (Microsoft Trusted Signing ou outro)**: preencher em
  `crates/native/src/install.rs`
  - `WINDOWS_PUBLISHER_CN = Some("CN=..., O=...")` (CN **e** O exatos do subject)
  - `WINDOWS_SIGNING_CA_PREFIX = Some("CN=... ")` (prefixo do issuer da CA)
  - Nunca fixar o thumbprint: a CA rotaciona o certificado a cada poucos dias.
  - Reforçar `PORTABLE_UPDATE_SCRIPT` em `crates/native/src/self_update.rs` com a
    mesma checagem (hoje: só `Valid` + certificado presente + thumbprint).
  - Os testes `only_the_publishers_valid_signature_is_trusted` e
    `the_portable_update_checks_the_new_executable_signature` cobrem os dois
    caminhos; estenda-os com o subject/issuer reais.
- **macOS (Apple Developer ID)**: preencher `macos::TEAM_ID = Some("...")` em
  `crates/native/src/install.rs`. `Expect::release` e `check` voltam a exigir a
  assinatura sozinhos. Teste real:
  `CRAFT_APP=/caminho/PhotoCraft.app TEAM_ID=... cargo test -p bd-launcher-native -- --ignored a_real_craft_app`.

## 2. Firebase: sem plano pago, sem Storage

O banco (RTDB) já tem as regras (`firebase deploy --only database` feito) e funciona
no plano gratuito. Para os arquivos, o desenho sem custo é:

- **Catálogo**: RTDB (`catalog.json`) — só metadados, sem `release` embutido.
- **Instaladores**: GitHub Releases de cada app + o fallback que o launcher já tem
  (`GitHubSource`: API, e sem quota via `releases/latest` + `SHA256SUMS.txt`). Todo
  download continua exigindo SHA-256 (`install::download` recusa sem checksum).
- **Ícones/fotos**: `raw.githubusercontent.com` (permitido em
  `catalog::storage_host_allowed`, com teste) — ficam nos repos dos apps, onde o web
  já os lê.

Se um dia quiser o bucket: console → projeto `bdesk-bdtecnologia` → **Storage →
Get Started** (pode pedir billing), depois
`firebase deploy --only storage --project bdesk-bdtecnologia`. Sem isso, nada do
Storage funciona — o plano acima não precisa dele.

## 3. GitHub: repositório próprio

O `origin` ainda aponta para o template (`storytold/craft-launcher`, sem escrita) e
`bd-tecnologia/bd-launcher` não existe. Passos:

1. Criar a org `bd-tecnologia` (ou escolher a conta) e o repo `bd-launcher`.
2. `git remote set-url origin https://github.com/<org>/bd-launcher.git && git push -u origin main`.
3. Registrar os secrets no repo novo (`FIREBASE_TOKEN`, `FIREBASE_SERVICE_ACCOUNT`,
   `APPLE_*`, `WINDOWS_*`, `AZURE_*`) — não migram sozinhos.
4. Repetir `FIREBASE_PROJECT`/`LAUNCHER_REPO` nas cópias de `publish.yml` dos repos
   de cada app.

## 4. Release: reapertar o cinto

Quando 1 e 3 estiverem prontos:

- Reativar os testes de assinatura nos CIs (Windows/macOS) com builds assinados.
- Auditoria WCAG + medição de Web Vitals no web (backlog não-bloqueante).
- `cargo audit`: hoje só restam avisos em `zip` (sem patch upstream; nosso uso não
  toca as APIs vulneráveis) e pacotes não mantidos via egui — reavaliar a cada release.
