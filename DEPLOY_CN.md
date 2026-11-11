# llm_wiki-web Docker 部署教程

本文按 2026-11-11 在 `swc-build` 上实际完成的部署编写，适用于已有 Docker 的 Linux x86_64 服务器。服务以单用户方式运行，通过 SSH 隧道访问。

当前实例已部署完成。日常访问从第 5 节开始；第 1～4 节用于新服务器，不要在现有实例上重复创建目录和容器。

## 1. 先理解部署结构

```text
工作电脑浏览器 http://127.0.0.1:18932
                  │ SSH 隧道
                  ▼
服务器 10.11.108.241 的 127.0.0.1:18932
                  │
容器 llmwiki-web-server
  /opt/llmwiki/bin/external-host   Rust 后端
  /opt/llmwiki/dist                原版网页及 Web 适配层
  /data                           项目与设置
                  │ 目录挂载
服务器 /home/hunt/llmwiki-web-data
```

镜像包含程序和运行库；容器是镜像启动后的进程；资料保存在服务器目录中。替换容器时继续挂载同一个目录即可保留资料，资料格式兼容性仍需检查。

服务器仓库 `~/llm_wiki-web` 用于保存源码和运维文件。运行容器使用镜像内的程序，并不直接读取仓库中的源码。`git pull` 不会自动更新正在运行的程序。

## 2. 检查服务器条件

在工作电脑执行：

```sh
ssh hunt@10.11.108.241
```

在服务器执行：

```sh
uname -m
docker version
docker info --format '{{.DockerRootDir}}'
id
df -h / /home
ss -ltnH '( sport = :18932 )'
```

要求：

- 架构为 `x86_64`，当前用户可执行 Docker 命令。
- Docker 的镜像目录和资料盘均有足够空间。镜像位于 DockerRootDir，资料和归档位于 `/home`；它们可能不是同一个磁盘。
- 18932 未被其他服务使用。工作电脑的同号端口也需要空闲。

本次服务器 Docker 已安装，未重新安装或修改 Docker daemon。实际 DockerRootDir 为 `/var/lib/docker`。

## 3. 准备并传输镜像与源码

### 3.1 本次采用现成镜像

在工作电脑执行。下面的路径是本次已经验证的镜像归档：

```sh
artifact_root=/mnt/BACKUP/llmwiki-clean-build-20261106/lifecycle-20261111/result
sha256sum "$artifact_root/llmwiki-web-lifecycle-linux-amd64.tar"
scp "$artifact_root/llmwiki-web-lifecycle-linux-amd64.tar" hunt@10.11.108.241:llmwiki-web-lifecycle-linux-amd64.tar
```

本次归档的 SHA-256：

```text
bf49f94d56971b05f1847ee2a136e7cbd986c82be806732dff0792b24ae91fae
```

在服务器执行并比较，必须完全一致：

```sh
sha256sum ~/llmwiki-web-lifecycle-linux-amd64.tar
```

服务器从归档载入程序，不需要安装 Rust、Node 或编译依赖，也不需要访问镜像仓库。

### 3.2 传输独立源码仓库

在工作电脑的新项目目录执行：

```sh
cd /home/hunt/work/llm_wiki-web
git status --short
git bundle create /tmp/llmwiki-server-main.bundle main
scp /tmp/llmwiki-server-main.bundle hunt@10.11.108.241:llmwiki-server-main.bundle
```

`git status --short` 应无输出。bundle 只包含已提交的 main 历史，不包含未提交文件或 `.runtime` 中的资料。

在服务器的新目录执行：

```sh
umask 077
git clone --branch main ~/llmwiki-server-main.bundle ~/llm_wiki-web
git -C ~/llm_wiki-web remote set-url origin https://github.com/uhuntu/llm_wiki-web.git
mkdir -p ~/llm_wiki-web/.runtime/deployment
mkdir -p ~/llmwiki-web-data
chmod 700 ~/llmwiki-web-data ~/llm_wiki-web/.runtime
mv ~/llmwiki-server-main.bundle ~/llm_wiki-web/.runtime/deployment/source.bundle
mv ~/llmwiki-web-lifecycle-linux-amd64.tar ~/llm_wiki-web/.runtime/deployment/image.tar
docker load -i ~/llm_wiki-web/.runtime/deployment/image.tar
```

已存在的仓库或资料目录应先检查，不能覆盖后再判断。

### 3.3 如果需要自行构建新版本

以下在构建电脑执行，输出目录需不存在，构建缓存和产物应放在空间充足的磁盘：

```sh
cd /home/hunt/work/llm_wiki-web
python3 prepare.py --output /path/on/build-disk/new-context
docker build -t llmwiki-web:my-release /path/on/build-disk/new-context
docker save -o /path/on/build-disk/llmwiki-web-my-release.tar llmwiki-web:my-release
```

`prepare.py` 获取固定上游提交，并将外置适配层组装成构建上下文。首次获取源码以及 Docker 基础镜像、软件包和依赖需要网络。离线已有上游 Git 仓库时可添加 `--source /path/to/upstream-repository`；这只解决源码获取，不等于全部构建依赖已离线准备好。

完整构建及代理、CA 参数见 [CONTAINER.md](CONTAINER.md)。新构建请使用新的镜像标签，传输后按第 8 节更新。本文的 lifecycle 标签专指已经验证的归档版本。

## 4. 启动独立容器

在服务器以普通用户 hunt 执行：

```sh
docker run -d \
  --name llmwiki-web-server \
  --label app=llm_wiki-web \
  --network host \
  --user "$(id -u):$(id -g)" \
  --restart unless-stopped \
  --read-only \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  --pids-limit 512 \
  --log-opt max-size=10m \
  --log-opt max-file=3 \
  --tmpfs /tmp:rw,nosuid,size=128m \
  --mount "type=bind,src=$HOME/llmwiki-web-data,dst=/data" \
  llmwiki-unchanged-host:lifecycle-20261111 \
  --mode live --port 18932
```

如果容器名称已存在，Docker 会拒绝创建；先检查现有容器，更新按第 8 节操作。

| 设置 | 原因 |
|---|---|
| `--network host` | 程序直接使用服务器网络；本机 Ollama 也可通过回环地址访问 |
| `--mode live` | 使用用户配置的真实模型；缺少配置会报错 |
| `--port 18932` | 程序自身监听 `127.0.0.1:18932` |
| `--user` | 使用当前普通用户的 UID/GID，能写自己拥有的资料目录 |
| `--read-only` | 程序目录只读，资料通过 `/data` 单独写入 |
| `--mount` | 将项目、聊天记录和设置保存在服务器真实目录 |
| `--tmpfs` | 临时文件放在最多 128 MiB 的内存文件系统中 |
| `--restart unless-stopped` | 进程退出、Docker 重启后可恢复；手动停用后保持停止 |
| 日志参数 | 每个日志文件最多 10 MiB，最多保留 3 个 |

这里没有 `-p`，因为使用 host 网络，监听范围由程序自身决定。不要将此命令改造成未经设计的公网入口。

本次已保存一个可重复启动的脚本：

```sh
~/llm_wiki-web/.runtime/deployment/run-server.sh
```

它启动已有容器，或在容器不存在时创建。更新版本仍需执行替换流程。

## 5. SSH 隧道与首次登录

在服务器查看：

```sh
docker logs --tail 20 llmwiki-web-server
ss -ltnH '( sport = :18932 )'
```

服务输出类似：

```text
LLM Wiki Web: http://localhost:18932/login?ticket=<一次性票据>
```

在工作电脑另开一个终端，保持以下命令运行：

```sh
ssh -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 \
  -L 127.0.0.1:18932:127.0.0.1:18932 hunt@10.11.108.241
```

把服务器输出的登录 URL 中 `localhost` 改成 `127.0.0.1`，在工作电脑浏览器打开：

```text
http://127.0.0.1:18932/login?ticket=<服务器输出的一次性票据>
```

- 票据只能使用一次；登录后浏览器通过 Cookie 保持会话，当前有效期为一小时。
- 必须保留同号端口 18932，服务会核对 Host 和 Origin。若该端口被占用，应协调两端使用同一个空闲端口。
- 使用 `127.0.0.1` 可以与工作电脑上通过 `localhost` 访问的另一实例分开保存 Cookie；Cookie 本身不按端口隔离。
- 隧道断开只会中断访问，服务器容器继续运行。按 Ctrl+C 可关闭前台隧道。

票据已使用或会话过期时，可由已登录会话请求新的 POST `/login-ticket`。当前尚无独立管理员登录 CLI。操作人员也可在确认没有导入、问答等任务运行时重启容器，然后使用最新启动日志中的新票据：

```sh
docker restart llmwiki-web-server
docker logs --tail 1 llmwiki-web-server
```

本次最初的票据已经用于部署验收，不能再次使用。不要将票据写入 Git 或分享给无关人员。

## 6. 验证与配置模型

服务器基础检查：

```sh
docker ps --filter name=llmwiki-web-server
docker inspect llmwiki-web-server --format 'user={{.Config.User}} readonly={{.HostConfig.ReadonlyRootfs}} privileged={{.HostConfig.Privileged}} restart={{.HostConfig.RestartPolicy.Name}}'
docker exec llmwiki-web-server python3 /opt/llmwiki/verify_source.py
curl -i http://127.0.0.1:18932/
```

最后一条未登录请求预期返回 401。再通过浏览器的一次性入口登录，确认能看见新工作区。

实际部署还验证了原版 Rust 文件写入、读取及容器重启后的保留。记录在服务器：

```text
~/llm_wiki-web/.runtime/deployment/verification.json
```

`verify_server.py` 是会重启容器、生成合成检查文件的部署验收脚本，不能当作定期无副作用健康检查运行。

打开 Settings → LLM Models，配置并激活模型。当前版本 Chat/Ingest 使用 Use active model；任务单独选模型的适配尚未完成。

使用服务器本机 Ollama 时，填写服务器的 `http://127.0.0.1:11434` 和已经安装的模型名称。远程模型服务需要其实际地址与凭证。配置后先用合成文本验证，不要仅凭连接测试成功判断 Agent 可用。

本次服务器部署尚未配置模型，也没有导入现有 DG3 文档。

## 7. 资料、备份与恢复

资料目录：

```text
~/llmwiki-web-data/
├─ external-ui-state.json      全局界面与模型设置
├─ .external-host-model-mode   工作区模式标记
└─ Web Wiki/                   项目目录
   ├─ wiki/                    Wiki 页面
   ├─ raw/                     原始资料
   └─ .llm-wiki/               聊天、缓存、队列等状态
```

上传和导出还会使用工作区中的 `.uploads` 与 `.downloads`。这些暂存文件目前需要人工管理。

做一致的目录备份前，先等待应用任务完成并停止容器。备份包含设置，可能包含模型凭证，应保持私密并按需离机保存：

```sh
umask 077
mkdir -p ~/llmwiki-web-backups
chmod 700 ~/llmwiki-web-backups
docker stop llmwiki-web-server
backup_file="$HOME/llmwiki-web-backups/workspace-$(date +%Y%m%d-%H%M%S).tar.gz"
tar -czf "$backup_file" -C "$HOME" llmwiki-web-data
tar -tzf "$backup_file" >/dev/null
docker start llmwiki-web-server
```

逐条检查命令结果。若打包或校验失败，不要把该文件当作有效备份。目前没有配置自动备份计划。

恢复时先停容器，将当前资料目录改名保留，再把选定备份解压到原来的父目录；确认目录所有者是运行容器的用户、权限为 0700，再启动。建议先在另一个目录和独立实例中检查备份。原版“Export project”只导出单个项目，不能替代整个工作区及全局设置备份。

## 8. 更新和回退

更新需要新的镜像归档和唯一标签。先传输、校验、load，记录旧镜像：

```sh
docker inspect llmwiki-web-server --format '{{.Image}}'
```

等待任务完成，按第 7 节备份。然后停止并保留旧容器：

```sh
docker stop llmwiki-web-server
docker rename llmwiki-web-server llmwiki-web-server-previous
```

`previous` 名称必须空闲。再使用第 4 节的完整命令创建容器，只替换镜像名称，继续挂载原资料目录。

检查启动、项目、模型设置和新旧功能。若需回退程序：

```sh
docker stop llmwiki-web-server
docker rename llmwiki-web-server llmwiki-web-server-failed
docker rename llmwiki-web-server-previous llmwiki-web-server
docker start llmwiki-web-server
```

上述名称同样必须空闲。回退容器不会撤销新版已写入的资料；若存在资料格式变化，应在停机状态下配合更新前的资料备份恢复。确认新版本稳定后，再按保留策略清理旧容器、镜像和归档。

## 9. 常见问题

| 现象 | 检查方式 |
|---|---|
| 浏览器连接失败 | 隧道是否运行、两端端口是否同号、容器是否启动 |
| Host/Origin rejected | 是否换了端口、使用了服务器 LAN IP 或其他主机名 |
| Login required | 是否使用新票据登录、Cookie 是否过期 |
| Invalid or used ticket | 同一票据已消费；获取新票据 |
| 无法写入资料 | bind mount 路径是否正确、UID/GID 和目录所有者是否一致 |
| 模型连接失败 | 服务器能否访问模型地址、模型名与凭证是否正确 |
| 导入完成却仍显示未导入 | 当前已知状态刷新问题，可用界面刷新按钮更新 |
| Docker 占用根盘空间 | 查看 DockerRootDir；资料目录在 /home 不代表镜像也在 /home |

## 10. 当前实例与后续接入范围

当前实例名 `llmwiki-web-server`，监听 `127.0.0.1:18932`。原来的 8090/8091 知识库服务和 8081/8082 Dependency-Track 没有调整。

现有 `~/dg3sw-kb/kb.sqlite` 是旧问答库，共享盘 `_Wiki` 也不是原版项目目录。它们没有挂载到新容器；接入前应明确转换与资料范围，先用独立副本验证。

当前已验证本地单用户部署、文本工作流和项目往返；多格式解析、任务模型路由、部分桌面服务及长期运维仍见 [ROADMAP.md](ROADMAP.md)。
