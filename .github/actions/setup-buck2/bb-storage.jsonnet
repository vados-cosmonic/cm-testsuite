// Loopback REAPI cache for Windows runners, where bazel-remote does not run.
// Each store is a fixed-size file, so the saved Actions cache entry is bounded.
local dir = std.extVar('BUCK_ACTION_CACHE_DIR');
local allow = { allow: {} };
local mebibyte = 1024 * 1024;

// A blob must fit in one block; blockBytes bounds the largest cacheable file.
local store(name, mapBytes, blockBytes) = {
  'local': {
    keyLocationMap: {
      onBlockDevice: { file: { path: dir + '/' + name + '.map', sizeBytes: mapBytes } },
      maximumGetAttempts: 16,
      maximumPutAttempts: 64,
    },
    oldBlocks: 4,
    currentBlocks: 8,
    newBlocks: 2,
    blocksOnBlockDevice: {
      source: { file: { path: dir + '/' + name + '.blocks', sizeBytes: 16 * blockBytes } },
      spareBlocks: 2,
    },
    // The Actions cache archives the store while the server is idle but running.
    persistent: {
      stateDirectoryPath: dir + '/' + name + '.state',
      minimumEpochInterval: '1s',
    },
  },
};

{
  grpcServers: [{
    listenAddresses: ['127.0.0.1:9092'],
    authenticationPolicy: allow,
  }],
  maximumMessageSizeBytes: 16 * mebibyte,
  contentAddressableStorageServer: {
    contentAddressableStorage: {
      chunkStorage: store('chunks', 8 * mebibyte, 256 * mebibyte),
      chunkMappingStorage: store('chunk-mappings', mebibyte, 4 * mebibyte),
    },
    getAuthorizer: allow,
    putAuthorizer: allow,
    findMissingAuthorizer: allow,
  },
  actionCache: {
    backend: store('actions', mebibyte, 4 * mebibyte),
    getAuthorizer: allow,
    putAuthorizer: allow,
  },
}
