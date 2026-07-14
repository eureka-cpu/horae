{ pkgs, flake, ... }:
# End-to-end OIDC login against a real (mock-connector) provider.
#
# Runs Horae with production OIDC enabled (no DEV_LOGIN) alongside a `dex`
# instance whose single `mockCallback` connector auto-authenticates a fixed
# identity (kilgore@kilgore.trout). This exercises the full authorization-code
# flow — /auth/login -> dex -> /auth/callback -> session — and asserts that a
# deactivated user is denied at sign-in (FR-002).
pkgs.testers.nixosTest {
  name = "horae-e2e-oidc";
  nodes.server = { config, ... }: {
    imports = [ flake.nixosModules.horae ];

    services.horae.enable = true;
    services.horae.database.createLocally = true;

    # Production auth: point Horae at the local dex issuer. DEV_LOGIN is unset,
    # so /auth/login starts the OIDC flow and the dev bypass is not registered.
    systemd.services.horae.environment = {
      HORAE_OIDC_ISSUER = "http://127.0.0.1:5556/dex";
      HORAE_OIDC_CLIENT_ID = "horae";
      HORAE_OIDC_CLIENT_SECRET = "horae-e2e-secret";
      HORAE_OIDC_REDIRECT_URL = "http://127.0.0.1:3000/auth/callback";
    };

    # Mock OIDC provider. The mockCallback connector auto-authenticates a fixed
    # verified identity; skipApprovalScreen keeps the flow headless.
    services.dex = {
      enable = true;
      settings = {
        issuer = "http://127.0.0.1:5556/dex";
        storage.type = "memory";
        web.http = "127.0.0.1:5556";
        oauth2.skipApprovalScreen = true;
        staticClients = [{
          id = "horae";
          name = "Horae";
          secret = "horae-e2e-secret";
          redirectURIs = [ "http://127.0.0.1:3000/auth/callback" ];
        }];
        connectors = [{
          type = "mockCallback";
          id = "mock";
          name = "Mock";
        }];
      };
    };

    # `horae` to seed/create users, `psql` to flip the active flag.
    environment.systemPackages = [ config.services.horae.package pkgs.postgresql ];
  };

  testScript = ''
    server.start()
    server.wait_for_unit("postgresql.service")
    server.wait_for_unit("dex.service")
    server.wait_for_unit("horae.service")
    server.wait_for_open_port(5556)
    server.wait_for_open_port(3000)

    # Health + provider discovery reachable
    server.succeed("curl -sf http://127.0.0.1:3000/health | grep -q ok")
    server.succeed("curl -sf http://127.0.0.1:5556/dex/.well-known/openid-configuration >/dev/null")

    # Seed the org, then create a member whose email matches the mock identity.
    server.succeed("sudo -u horae DATABASE_URL=postgres:///horae horae seed")
    server.succeed(
      "sudo -u horae DATABASE_URL=postgres:///horae horae user create "
      "--email kilgore@kilgore.trout --name 'Kilgore Trout' --role member"
    )

    # Drive the full authorization-code flow with a cookie jar, following every
    # redirect: /auth/login -> dex (mock auto-auth) -> /auth/callback -> /.
    server.succeed(
      "curl -s -c /tmp/jar.txt -b /tmp/jar.txt -L "
      "http://127.0.0.1:3000/auth/login -o /dev/null"
    )

    # Session established: an authenticated request now succeeds.
    result = server.succeed(
      "curl -s -b /tmp/jar.txt http://127.0.0.1:3000/harvest/v2/time_entries"
    )
    assert '"time_entries"' in result, f"expected authenticated Harvest response, got: {result[:200]}"

    # FR-002: deactivate the user; a fresh OIDC login must NOT establish a session.
    server.succeed(
      "sudo -u horae psql horae -c "
      "\"UPDATE users SET active = false WHERE email = 'kilgore@kilgore.trout'\""
    )
    server.succeed(
      "curl -s -c /tmp/jar2.txt -b /tmp/jar2.txt -L "
      "http://127.0.0.1:3000/auth/login -o /dev/null"
    )
    code = server.succeed(
      "curl -s -o /dev/null -w '%{http_code}' -b /tmp/jar2.txt "
      "http://127.0.0.1:3000/harvest/v2/time_entries"
    ).strip()
    assert code != "200", f"deactivated user must not hold an authenticated session, got {code}"
  '';
}
