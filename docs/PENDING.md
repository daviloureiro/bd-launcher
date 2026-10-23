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

## 2. Firebase: tudo no plano gratuito (sem billing)

O banco (RTDB) já tem as regras (`firebase deploy --only database` feito) e funciona
no plano gratuito. Para os arquivos, o desenho sem custo é:

- **RTDB**: 1 GB guardado + 10 GB/mês de download grátis. Regras já deployadas. ✓
- **Storage**: 5 GB guardados + 1 GB/dia de download grátis no Spark — **sem pagar**.
  O que travou foi só a criação do bucket (a API crua exige billing; o botão
  **Get Started** no console cria dentro da cota gratuita). 1 clique seu, depois
  eu rodo o deploy das regras daqui.
- **Instaladores grandes**: se 1 GB/dia estourar, binários vão para GitHub Releases
  de cada app + fallback nativo (`GitHubSource`, sem quota prática), sempre com
  SHA-256 obrigatório.
- **Ícones/fotos**: Storage ou `raw.githubusercontent.com` (permitido, com teste).
- **SQL**: Firebase só tem SQL via Data Connect/Cloud SQL em **trial de 3 meses** —
  depois vira Blaze automático. Sem plano pago permanente, e não precisamos:
  catálogo é JSON (RTDB), arquivos são blobs. SQL = custo + complexidade + risco
  de cobrança surprise.

## 3. GitHub: repositório próprio

O `origin` agora é o repo próprio (`daviloureiro/bd-launcher`) e os workflows
compartilhados vêm do fork próprio (`daviloureiro/craft-common-actions`). Falta:

1. (Opcional) criar a org `bd-tecnologia` e transferir o repo para ela
   (Settings → Transfer ownership); aí trocar `daviloureiro/` por
   `bd-tecnologia/` em `Cargo.toml`, `publish.yml`, `README.md`, `AGENTS.md`,
   `packaging/` e nos `uses:` dos workflows.
2. Registrar os secrets no repo (`FIREBASE_TOKEN`, `FIREBASE_SERVICE_ACCOUNT`,
   `APPLE_*`, `WINDOWS_*`, `AZURE_*`) — não migram sozinhos.
3. Repetir `FIREBASE_PROJECT`/`LAUNCHER_REPO` nas cópias de `publish.yml` dos repos
   de cada app, quando eles existirem — e trocar o prefixo `bd-tecnologia/` pelos
   repos reais em `README.md` (tabela dos apps), `ATTRIBUTION.md`, `GITHUB_URL`
   (`catalog.rs`) e nas fixtures que usam a org como exemplo.
4. Domínio canônico: `bdtecnolog.ia.br` (resolvido). `WEBSITE_URL`, README e metainfo
   (`homepage`, `contact` → WhatsApp, developer id `br.ia.bdtecnolog`) unificados nele.

## 4. Release: reapertar o cinto

Quando 1 e 3 estiverem prontos:

- Reativar os testes de assinatura nos CIs (Windows/macOS) com builds assinados.
- Auditoria WCAG + medição de Web Vitals no web (backlog não-bloqueante).
- `cargo audit`: hoje só restam avisos em `zip` (sem patch upstream; nosso uso não
  toca as APIs vulneráveis) e pacotes não mantidos via egui — reavaliar a cada release.
