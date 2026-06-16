module.exports = {
  apps: [{
    name: 'ghosthanddesk',
    script: '/opt/ghosthanddesk/server/signaling-server',
    env: {
      REQUIRE_TLS: 'false',
      DISABLE_ORIGIN_CHECK: 'true',
      SERVER_HOST: ':9000',
      MAX_CLIENTS: '1000',
      LOG_LEVEL: 'info',
      CONNECTION_TIMEOUT: '60'
    },
    log_file: '/var/log/pm2/ghosthanddesk.log',
    error_file: '/var/log/pm2/ghosthanddesk-error.log',
    out_file: '/var/log/pm2/ghosthanddesk-out.log',
    restart_delay: 3000,
    max_restarts: 10
  }]
}
