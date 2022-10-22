import os
import sys

def get_help():
    """
    Sets up a local Docker Nginx proxy which caches requests to a specific server.

    python3 create_proxy CACHE_PATH UPSTREAM_URL PORT

    CACHE_PATH   - a path to a FOLDER where the cache will be
    UPSTREAM_URL - the URL of the server whose responses will be cached
    PORT         - the port on which the proxy will listen ON LOCALHOST
    """

def main():
    if len(sys.argv) < 4:
        print("Invalid argument count", file=sys.stderr)
        sys.exit(1)
    
    try:
        cache_path = os.path.abspath(sys.argv[1])
    except:
        print("Supplied path is not in correct format")
        sys.exit(1)


    upstream_server_url = sys.argv[2]
    port = -1
    try:
        port = int(sys.argv[3])
        if port < 1024 or port > 65535:
            raise ValueError("Invalid port")
    except:
        print("Supplied port is not a number or an invalid number")
        sys.exit(1)

    create_cache_folder(cache_path)
    create_proxy_config(upstream_server_url)
    create_docker_compose(cache_path, port)



def create_cache_folder(folder_path):
    if not os.path.exists(folder_path):
        os.makedirs(folder_path)

def create_proxy_config(upstream_url):
    file_contents = """
worker_processes  1;

events {
    worker_connections  1024;
}

http {
	log_format custom_cache_log '[$time_local] [Cache:$upstream_cache_status] [$host] [Remote_Addr: $remote_addr] - $remote_user - $server_name to: $upstream_addr: "$request" $status $body_bytes_sent "$http_referer" "$http_user_agent" ' ;

    proxy_cache_path  /tmp/proxy_cache  levels=1:2 keys_zone=my_cache:10m inactive=168h  max_size=1g;
    server {
		access_log  /var/log/nginx/example.com.log custom_cache_log ;
        listen       5000;
        location / {
			proxy_set_header Host osu.ppy.sh;
			proxy_buffering        on;
            proxy_ssl_server_name on;
			proxy_ignore_headers Cache-Control Expires X-Accel-Expires;
			proxy_ignore_headers Set-Cookie;
			proxy_cache            my_cache;
			proxy_cache_valid      200  999d;
            proxy_pass             {0};
        }
    }
}    
"""
    file_contents = file_contents.replace("{0}", upstream_url)
    with open("./nginx.conf", "w") as f:
        f.write(file_contents)

def create_docker_compose(cache_path, port):
    file_contents = """
version: '3.1'
services:
    my-proxy:
      image: nginx
      volumes:
        - ./nginx.conf:/etc/nginx/nginx.conf:ro
        - {0}:/tmp/proxy_cache:rw
      ports:
        - 127.0.0.1:{1}:5000
"""
    file_contents = file_contents.replace("{0}", cache_path).replace("{1}", str(port))
    with open("./docker-compose.yml", "w") as f:
        f.write(file_contents)

main()