get_zulucrypt_device() {
    for device in /dev/mapper/zuluCrypt*; do
        device_path=$(zuluCrypt-cli -P -d "$device")
        [ "$device_path" = "$1" ] && echo "$device" && return 0
    done
    return 1
}

echo $PASSWORD | zuluCrypt-cli -o -d $PATH -t vcrypt
mount -t ntfs -o permissions,windows_names /dev/mappter/zuluCrypt-$(get_zulucrypt_device $PATH) $MOUNT
