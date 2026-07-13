[English](../en/Design-Cache.md)
[返回首页](Home.md)

# Cache 设计

每个内部 `Cache` 管理一个 Apollo 命名空间，并组合带时间戳的内存值、原生文件或浏览器 localStorage、远程获取和有序监听器。

## 状态与身份

- `memory: Arc<RwLock<Option<CacheItem>>>` 同时保存 JSON 值和获取时间，因此 `cache_ttl` 同时约束内存和持久缓存。
- `load_lock: Arc<Mutex<()>>` 是取消安全的冷启动/过期加载单航班锁。
- 原生文件名为 `v2-{sha1(identity)}.cache.json`，WASM 键为 `apollo_cache_v2_{sha1(identity)}`。身份包含服务器、应用、集群、命名空间、IP 和标签，原始标识符不会成为路径组件。

## 读取与刷新

`get_value()` 优先返回新鲜内存值。未命中时获取 `load_lock` 并再次检查，然后读取磁盘或 localStorage。持久值过期时请求 Apollo；请求失败则返回最新的已解析陈旧值（如果存在）。

只有成功 HTTP 响应才会解析和缓存。持久写入是尽力而为：目录不可写或 localStorage 不可用只记录警告，不会丢弃有效远程响应。

`refresh()` 在不持有内存写锁的情况下执行网络和持久 I/O，仅在原子替换值时短暂写锁，因此慢请求不会阻塞读取。取消加载任务会自动释放互斥锁。

原生写入使用目标目录中的唯一临时文件，刷新后原子重命名，支持并发最后写入者胜出。

## 监听器与错误

监听器在内部锁释放后按注册顺序同步运行。只有配置值发生变化才发送成功事件；刷新失败发送 `Error::Refresh`。监听器 panic 会被捕获并记录。

网络、超时、非成功 HTTP 状态、URL、签名和 JSON 解析错误均保留为类型化错误；持久缓存读写错误是可观测警告而不是配置获取失败。
