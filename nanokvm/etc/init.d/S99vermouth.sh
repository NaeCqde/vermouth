#!/bin/sh
case "$1" in
  start)
        echo "Start Vermouth Client for NanoKVM"
        cd /data/vermouth
        ./vermouth-nanokvm
        ;;
  stop)
        ;;
  restart|reload)
        "$0" stop
        "$0" start
        ;;
  *)
        echo "Usage: $0 {start|stop|restart}"
        exit 1
esac
exit $?
