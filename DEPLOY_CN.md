# llm_wiki-web Docker 部署教程

本文适用于已有 Docker 的 Linux x86_64 服务器，采用单用户容器和 SSH 隧道访问。其他架构需要构建对应架构的镜像。

首次部署按顺序执行；已有实例的日常访问从第 5 节开始。具体环境的部署记录见独立运维文档。

## 使用前填写变量

以下地址和名称是占位示例，请替换成自己的环境。工作电脑和服务器的 shell 变量不会通过 SSH 自动传递；请在各自终端分别设置。

在工作电脑设置：

```sh
server_host=server.example.com
server_user=alice
web_port=18932
source_dir=/path/to/llm_wiki-web
image_name=llmwiki-web:YOUR_RELEASE
image_archive=/path/to/llmwiki-web-YOUR_RELEASE.tar
```

在服务器的普通用户终端设置：

```sh
web_port=18932
container_name=llmwiki-web
image_name=llmwiki-web:YOUR_RELEASE
project_dir="$HOME/llm_wiki-web"
data_dir="$HOME/llmwiki-web-data"
deploy_dir="$project_dir/.runtime/deployment"
backup_dir="$HOME/llmwiki-web-backups"
```

`image_name` 必须与实际构建或交付的镜像标签一致。`YOUR_RELEASE` 不是现成发布标签；本文不假设有公共镜像仓库。`web_port` 在工作电脑与服务器两端应一致，且都未被占用，取值范围为 1024～65535。目录变量应使用绝对路径。每次打开新终端或重新 SSH 登录，都要重新设置该端所需变量。


## 1. 先理解部署结构

```text
工作电脑浏览器 http://127.0.0.1:<web_port>
                  │ SSH 隧道
                  ▼
目标服务器的 127.0.0.1:<web_port>
                  │
容器 <container_name>
  /opt/llmwiki/bin/external-host   Rust 后端
  /opt/llmwiki/dist                原版网页及 Web 适配层
  /data                           项目与设置
                  │ 目录挂载
服务器 <data_dir>
```

镜像包含程序和运行库；容器是镜像启动后的进程；资料保存在服务器目录中。替换容器时继续挂载同一个目录即可保留资料，资料格式兼容性仍需检查。

服务器仓库 `$project_dir` 用于保存源码和运维文件。运行容器使用镜像内的程序，并不直接读取仓库中的源码。`git pull` 不会自动更新正在运行的程序。

## 2. 检查服务器条件

在工作电脑执行：

```sh
ssh "$server_user@$server_host"
```

在服务器执行：

```sh
uname -m
docker version
docker info --format '{{.DockerRootDir}}'
id
df -h / /home
ss -ltnH "( sport = :$web_port )"
```

要求：

- 架构为 `x86_64`，当前用户可执行 Docker 命令。
- Docker 的镜像目录和资料盘均有足够空间。镜像位于 DockerRootDir，资料和归档的位置由所选目录决定；它们可能不是同一个磁盘。
- 选定的 `web_port` 未被其他服务使用。工作电脑的同号端口也需要空闲。

本文从 Docker 已安装且 daemon 正常运行开始。DockerRootDir 以实际查询结果为准。

## 3. 准备并传输镜像与源码

### 3.1 准备镜像并传输

可以使用可信交付方提供的镜像归档，也可以按第 3.3 节自行构建。确认镜像架构适合服务器，然后在工作电脑执行：

```sh
sha256sum "$image_archive"
scp "$image_archive" "$server_user@$server_host:llmwiki-web-image.tar"
```

记录上一步的哈希；若使用交付归档，应先与交付方提供的校验值比较。服务器执行：

```sh
sha256sum ~/llmwiki-web-image.tar
```

传输两端的哈希必须一致。不要直接套用其他版本、其他人的镜像哈希。

服务器从归档载入程序，不需要安装 Rust、Node 或编译依赖，也不需要访问镜像仓库。

### 3.2 传输独立源码仓库

工作电脑需先取得此项目的 Git 仓库，并把 `source_dir` 指向该仓库根目录。在该目录执行：

```sh
cd "$source_dir"
git status --short
git bundle create /tmp/llmwiki-server-main.bundle main
scp /tmp/llmwiki-server-main.bundle "$server_user@$server_host:llmwiki-server-main.bundle"
```

`git status --short` 应无输出。bundle 只包含已提交的 main 历史，不包含未提交文件或 `.runtime` 中的资料。

在服务器的新目录执行：

```sh
umask 077
git clone --branch main ~/llmwiki-server-main.bundle "$project_dir"
git -C "$project_dir" remote remove origin
# 如需远程同步，可另行添加自己的仓库地址：
# git -C "$project_dir" remote add origin YOUR_REPOSITORY_URL
mkdir -p "$deploy_dir" "$data_dir"
chmod 700 "$data_dir" "$project_dir/.runtime"
mv ~/llmwiki-server-main.bundle "$deploy_dir/source.bundle"
mv ~/llmwiki-web-image.tar "$deploy_dir/image.tar"
docker load -i "$deploy_dir/image.tar"
docker image inspect "$image_name" --format '{{.Os}}/{{.Architecture}} {{.Id}}'
```

已存在的仓库或资料目录应先检查，不能覆盖后再判断。

### 3.3 如果需要自行构建新版本

以下在与目标服务器架构一致的 Linux 构建电脑执行，输出目录需不存在，构建缓存和产物应放在空间充足的磁盘。跨架构构建需要另行准备模拟或原生构建节点，本文不涵盖：

```sh
cd "$source_dir"
python3 prepare.py --output /path/on/build-disk/new-context
docker build -t "$image_name" /path/on/build-disk/new-context
docker save -o "$image_archive" "$image_name"
```

`prepare.py` 获取固定上游提交，并将外置适配层组装成构建上下文。首次获取源码以及 Docker 基础镜像、软件包和依赖需要网络。离线已有上游 Git 仓库时可添加 `--source /path/to/upstream-repository`；这只解决源码获取，不等于全部构建依赖已离线准备好。

完整构建及代理、CA 参数见 [CONTAINER.md](CONTAINER.md)。新构建请使用新的唯一镜像标签，传输后按第 8 节更新。

## 4. 启动独立容器

在服务器以拥有资料目录的普通用户执行：

```sh
docker run -d \
  --name "$container_name" \
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
  --mount "type=bind,src=$data_dir,dst=/data" \
  "$image_name" \
  --mode live --port "$web_port"
```

如果容器名称已存在，Docker 会拒绝创建；先检查现有容器，更新按第 8 节操作。

| 设置 | 原因 |
|---|---|
| `--network host` | 程序直接使用服务器网络；本机 Ollama 也可通过回环地址访问 |
| `--mode live` | 使用用户配置的真实模型；缺少配置会报错 |
| `--port` | 程序自身监听选定端口的 `127.0.0.1` 地址 |
| `--user` | 使用当前普通用户的 UID/GID，能写自己拥有的资料目录 |
| `--read-only` | 程序目录只读，资料通过 `/data` 单独写入 |
| `--mount` | 将项目、聊天记录和设置保存在服务器真实目录 |
| `--tmpfs` | 临时文件放在最多 128 MiB 的内存文件系统中 |
| `--restart unless-stopped` | 进程退出、Docker 重启后可恢复；手动停用后保持停止 |
| 日志参数 | 每个日志文件最多 10 MiB，最多保留 3 个 |

这里没有 `-p`，因为使用 host 网络，监听范围由程序自身决定。不要将此命令改造成未经设计的公网入口。

建议将自己的完整启动命令保存为部署脚本，供重建容器时使用。不要仅依赖终端历史；启动现有容器可以用 `docker start "$container_name"`。

## 5. SSH 隧道与首次登录

在服务器查看：

```sh
docker logs --tail 20 "$container_name"
ss -ltnH "( sport = :$web_port )"
```

服务输出类似：

```text
LLM Wiki Web: http://localhost:<web_port>/login?ticket=<一次性票据>
```

在工作电脑另开一个终端，重新设置 `server_host`、`server_user` 和 `web_port`，保持以下命令运行：

```sh
ssh -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 \
  -L "127.0.0.1:$web_port:127.0.0.1:$web_port" "$server_user@$server_host"
```

把服务器输出的登录 URL 中 `localhost` 改成 `127.0.0.1`，在工作电脑浏览器打开：

```text
http://127.0.0.1:<web_port>/login?ticket=<服务器输出的一次性票据>
```

- 票据只能使用一次；登录后浏览器通过 Cookie 保持会话，当前有效期为一小时。
- 必须保留两端同号端口，服务会核对 Host 和 Origin。若该端口被占用，应协调两端使用同一个空闲端口。
- 使用 `127.0.0.1` 可以与工作电脑上通过 `localhost` 访问的另一实例分开保存 Cookie；Cookie 本身不按端口隔离。
- 隧道断开只会中断访问，服务器容器继续运行。按 Ctrl+C 可关闭前台隧道。

票据已使用或会话过期时，可由已登录会话请求新的 POST `/login-ticket`。当前尚无独立管理员登录 CLI。操作人员也可在确认没有导入、问答等任务运行时重启容器，然后使用最新启动日志中的新票据：

```sh
docker restart "$container_name"
docker logs --tail 1 "$container_name"
```

已使用的票据不能再次使用。不要将票据写入 Git 或分享给无关人员。

## 6. 验证与配置模型

服务器基础检查：

```sh
docker ps --filter "name=$container_name"
docker inspect "$container_name" --format 'user={{.Config.User}} readonly={{.HostConfig.ReadonlyRootfs}} privileged={{.HostConfig.Privileged}} restart={{.HostConfig.RestartPolicy.Name}}'
docker exec "$container_name" python3 /opt/llmwiki/verify_source.py
curl -i "http://127.0.0.1:$web_port/"
```

最后一条未登录请求预期返回 401。再通过浏览器的一次性入口登录，确认能看见新工作区。

上线前还应在独立测试项目里验证：文件写入读取、浏览器上传与导入、项目导出，以及重启后的数据保留。将镜像标签、Git 提交、校验值和结果保存为环境运维记录。

打开 Settings → LLM Models，配置并激活模型。当前版本 Chat/Ingest 使用 Use active model；任务单独选模型的适配尚未完成。

使用服务器本机 Ollama 时，填写服务器的 `http://127.0.0.1:11434` 和已经安装的模型名称。远程模型服务需要其实际地址与凭证。配置后先用合成文本验证，不要仅凭连接测试成功判断 Agent 可用。

模型与资料都需要由部署者配置；镜像不会自动接入服务器上已有的知识库。

## 7. 资料、备份与恢复

资料目录：

```text
<data_dir>/
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
mkdir -p "$backup_dir"
chmod 700 "$backup_dir"
docker stop "$container_name"
backup_file="$backup_dir/workspace-$(date +%Y%m%d-%H%M%S).tar.gz"
tar -czf "$backup_file" -C "$(dirname "$data_dir")" "$(basename "$data_dir")"
tar -tzf "$backup_file" >/dev/null
docker start "$container_name"
```

逐条检查命令结果。若打包或校验失败，不要把该文件当作有效备份。定期备份、离机保存和恢复演练需要部署者另行配置。

恢复时先停容器，将当前资料目录改名保留，再把选定备份解压到原来的父目录；确认目录所有者是运行容器的用户、权限为 0700，再启动。建议先在另一个目录和独立实例中检查备份。原版“Export project”只导出单个项目，不能替代整个工作区及全局设置备份。

## 8. 更新和回退

更新需要新的镜像归档和唯一标签。先传输、校验、load，记录旧镜像：

```sh
docker inspect "$container_name" --format '{{.Image}}'
```

等待任务完成，按第 7 节备份。然后停止并保留旧容器：

```sh
docker stop "$container_name"
docker rename "$container_name" "${container_name}-previous"
```

`${container_name}-previous` 名称必须空闲。再使用第 4 节的完整命令创建容器，只替换镜像名称，继续挂载原资料目录。

检查启动、项目、模型设置和新旧功能。若需回退程序：

```sh
docker stop "$container_name"
docker rename "$container_name" "${container_name}-failed"
docker rename "${container_name}-previous" "$container_name"
docker start "$container_name"
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

## 10. 接入现有资料与适用范围

部署时先使用新的工作区，并保持已有服务的端口、目录和计划任务独立。已有 Markdown 库、SQLite 问答数据库或共享盘目录不一定是原版 llm_wiki 项目。接入前先确认项目结构、资料范围与转换方式，再用独立副本验证。

目前工程已验证 Linux x86_64 的单用户回环部署、文本工作流、项目导入导出及持久化。这些结果不代表任意文档格式、模型、架构或公网部署都已验证。剩余工程范围见 [ROADMAP.md](ROADMAP.md)。
