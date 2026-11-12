// Loopback REAPI cache for Windows runners, where bazel-remote does not run.
// Each store is a fixed-size file, so the saved Actions cache entry is bounded.
local dir = std.extVar('BUCK_ACTION_CACHE_DIR');
local allow = { allow: {} };
local mebibyte = 1024 * 1024;

// A blob must fit in one block; blockBytes bounds the largest cacheable file.
// Stores whose entries can be overwritten accept only one "new" block.
local store(name, mapBytes, blockBytes, newBlocks) = {
  'local': {
    keyLocationMap: {
      onBlockDevice: { file: { path: dir + '/' + name + '.map', sizeBytes: mapBytes } },
      maximumGetAttempts: 16,
      maximumPutAttempts: 64,
    },
    oldBlocks: 4,
    currentBlocks: 10 - newBlocks,
    newBlocks: newBlocks,
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
      // The server splits blobs into chunks and refuses to store them otherwise.
      chunkStorage: store('chunks', 8 * mebibyte, 256 * mebibyte, 2) + {
        'local'+: {
          chunkingParameters: {
            minChunkSizeBytes: 256 * 1024,
            horizonSizeBytes: 8 * 256 * 1024,
          },
        },
      },
      chunkMappingStorage: store('chunk-mappings', mebibyte, 4 * mebibyte, 1),
    },
    getAuthorizer: allow,
    putAuthorizer: allow,
    findMissingAuthorizer: allow,
  },
  actionCache: {
    backend: store('actions', mebibyte, 4 * mebibyte, 1),
    getAuthorizer: allow,
    putAuthorizer: allow,
  },
}
