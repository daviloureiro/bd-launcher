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

## 2. Firebase: RTDB + GitHub Releases (sem Storage, sem SQL pago)

O catálogo é um JSON no **Realtime Database** (1 GB guardado + 10 GB/mês de
download, grátis) e os arquivos ficam nos **GitHub Releases** de cada app. As
regras já estão deployadas (`firebase deploy --only database`) e
`/catalog.json` responde 200. Nada aqui usa Storage nem SQL:

- **Catálogo**: RTDB, `/catalog.json`. ✓ deployado; hoje vazio.
- **Instaladores**: assets do GitHub Release do próprio app. O `publish` grava
  a URL absoluta em `release.base` com `--base-url` (e `--no-upload`), então o
  launcher baixa direto de `github.com/<repo>/releases/download/...`.
- **Ícones/fotos**: `raw.githubusercontent.com` do repo do app (host permitido
  em `catalog::storage_host_allowed`, com teste).
- **Storage**: **não é necessário**. O `firebase.json` não declara mais Storage,
  então `firebase deploy` não tenta criar bucket. O upload para bucket continua
  existindo como opção (`cargo xtask publish` sem `--base-url`), mas nenhum
  fluxo do projeto depende dele.
- **SQL**: Firebase só tem SQL via Data Connect/Cloud SQL em **trial de 3 meses** —
  depois vira Blaze automático. Não precisamos: o catálogo é um documento e os
  arquivos são blobs.
- **Download público**: os repos dos apps precisam ser **públicos** (o
  `bd-launcher` já é). Asset de repo privado exige token e o launcher não tem
  como autenticar no GitHub.

## 3. GitHub: repositório próprio

O `origin` é o repo próprio (`daviloureiro/bd-launcher`), **público** — os
downloads dos GitHub Releases dependem disso. Os workflows compartilhados vêm do
fork próprio (`daviloureiro/craft-common-actions`). Falta:

1. (Opcional) criar a org `bd-tecnologia` e transferir o repo para ela
   (Settings → Transfer ownership); aí trocar `daviloureiro/` por `bd-tecnologia/`
   em `Cargo.toml`, `publish.yml`, `README.md`, `AGENTS.md`, `packaging/`, nos
   `uses:` dos workflows e em `catalog::GITHUB_URL` / `App::github` do launcher.
   A org **não existe hoje**; por isso o código aponta para `daviloureiro`.
2. Registrar o único secret que o publish precisa: `FIREBASE_SERVICE_ACCOUNT`
   (JSON da conta de serviço — *Project settings → Service accounts*). O antigo
   `FIREBASE_TOKEN`/`firebase login:ci` saiu do workflow: ele é interativo e não
   funciona em CI; o token do banco agora vem do `google-github-actions/auth`.
3. Repetir `FIREBASE_PROJECT`/`LAUNCHER_REPO` nas cópias de `publish.yml` dos repos
   de cada app, quando eles existirem — e trocar o prefixo `bd-tecnologia/` pelos
   repos reais em `README.md` (tabela dos apps), `ATTRIBUTION.md` e nas fixtures
   que usam a org como exemplo.
4. Domínio canônico: `bdtecnolog.ia.br` (resolvido). `WEBSITE_URL`, README e metainfo
   (`homepage`, `contact` → WhatsApp, developer id `br.ia.bdtecnolog`) unificados nele.

## 4. Release: reapertar o cinto

**Os certificados são obrigatórios?** Não para testar; sim para distribuir em
escala. Eles servem para provar que o instalador saiu da BD: sem o pin, o
launcher aceita qualquer binário com uma assinatura válida (inclusive de outro
desenvolvedor); com o pin, só aceita o seu publisher. Enquanto não houver
certificado, os testes de assinatura ficam desligados e o launcher confia na
validação genérica do sistema.

Quando 1 e 3 estiverem prontos:

- Reativar os testes de assinatura nos CIs (Windows/macOS) com builds assinados.
- Auditoria WCAG + Web Vitals do web (medido em 2026-10-23, build trunk 0.21.14 idêntico
  ao CI, via Lighthouse local): acessibilidade 100, boas práticas 100, SEO 100
  (após `meta description`); wasm 9,1 MiB, abaixo do gate de 24 MiB.
- `cargo audit`: hoje só restam avisos em `zip` (sem patch upstream; nosso uso não
  toca as APIs vulneráveis) e pacotes não mantidos via egui — reavaliar a cada release.
