#!/bin/bash

set -e

OUTPUTFILE=$(mktemp)

resim reset

echo
resim new-account >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
export account=$(grep 'Account component address:' $OUTPUTFILE | cut -d ' ' -f 4)
export owner_badge=$(grep 'Owner badge:' $OUTPUTFILE | cut -d ':' -f 2)
echo Account address: $account
echo Owner badge: $owner_badge

echo
resim publish . >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
export package=$(grep 'Success! New Package:' $OUTPUTFILE | cut -d ' ' -f 4)
echo Package: $package

resim call-function ${package} BookingSystem new ${owner_badge} >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
export component=$(grep 'Component:' $OUTPUTFILE | cut -d ' ' -f 3)
export user_badge=$(grep 'Resource:' $OUTPUTFILE | tail -n 3 | head -n 1 | cut -d ' ' -f 3)
export reservation=$(grep 'Resource:' $OUTPUTFILE | tail -n 2 | head -n 1 | cut -d ' ' -f 3)
export arbitrator_badge=$(grep 'Resource:' $OUTPUTFILE | tail -n 1 | cut -d ' ' -f 3)
echo Component address: $component
echo User badge: $user_badge
echo Reservation NFT: $reservation
echo Arbitrator badge: $arbitrator_badge

echo
resim call-method ${component} new_user >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
export user_id=$(grep -A 1 "ResAddr: $user_badge" $OUTPUTFILE | tail -n 1 | cut -d '#' -f 2)
echo "User created, NFT #${user_id}# received (should be 1)"

echo
export minimum_reservation_period=86400 # 1 day
export accepted_coin_address=resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3 # XRD
export min_cancellation_forewarning=604800 # 1 week
resim run manifests/new_item.rtm >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
export item_id=$(grep item_id: $OUTPUTFILE | head -n 1 | cut -d ':' -f 2 | cut -d 'u' -f 1)
echo "Item ${item_id} created (shoud be 1)"

echo
export start_time=1735689600 # Wed Jan 01 2025 00:00:00 GMT+0000
export available=true
export price_per_minimum_reservation_period=10
resim run manifests/add_or_modify_availability_interval.rtm >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
echo Set price for item ${item_id} $price_per_minimum_reservation_period since $start_time

export start_time=1736208000 # Tue Jan 07 2025 00:00:00 GMT+0000
export price_per_minimum_reservation_period=5
resim run manifests/add_or_modify_availability_interval.rtm >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
echo Set price or item ${item_id} $price_per_minimum_reservation_period since $start_time

export start_time=1736899200 # Wed Jan 15 2025 00:00:00 GMT+0000
export available=false
resim run manifests/add_or_modify_availability_interval.rtm >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
echo Set item ${item_id} not available since $start_time

echo
export start_time=1735689600 # Wed Jan 01 2025 00:00:00 GMT+0000
export end_time=1736467200 # Fri Jan 10 2025 00:00:00 GMT+0000
export payment_amount=$(( ((1736208000 - 1735689600) * 10 + (1736467200 - 1736208000) * 5) / $minimum_reservation_period - 1 ))
resim run manifests/new_reservation.rtm >$OUTPUTFILE && ( echo "This transaction was supposed to fail!" ; exit 1 )
echo "Reservation from ${start_time} to ${end_time} failed for insufficient payment (${payment_amount})"

echo
export payment_amount=$(( ((1736208000 - 1735689600) * 10 + (1736467200 - 1736208000) * 5) / $minimum_reservation_period ))
resim run manifests/new_reservation.rtm >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
export reservation_id=$(grep -A 1 "ResAddr: $reservation" $OUTPUTFILE | tail -n 1 | cut -d '#' -f 2)
echo "Reservation ${reservation_id} booked from ${start_time} to ${end_time}, depositing ${payment_amount} XRD"

echo
export start_time=1736380800 # Thu Jan 09 2025 00:00:00 GMT+0000
export end_time=1736553600 # Sat Jan 11 2025 00:00:00 GMT+0000
resim run manifests/new_reservation.rtm >$OUTPUTFILE && ( echo "This transaction was supposed to fail!" ; exit 1 )
echo "New reservation for overlapping period (${start_time} to ${end_time}) failed"

echo
export start_time=1736553600 # Sat Jan 11 2025 00:00:00 GMT+0000
export end_time=1736985600 # Thu Jan 16 2025 00:00:00 GMT+0000
resim run manifests/new_reservation.rtm >$OUTPUTFILE && ( echo "This transaction was supposed to fail!" ; exit 1 )
echo "New reservation for period of item unavailability (${start_time} to ${end_time}) failed"

echo
export cancellation_time=2024-12-26T00:00:00Z
resim set-current-time $cancellation_time
resim call-method ${component} reservation_cancellation_by_customer ${reservation}:${reservation_id} >$OUTPUTFILE && ( echo "This transaction was supposed to fail!" ; exit 1 )
echo "Cancellation attempt by the customer for reservation ${reservation_id} on ${cancellation_time} failed"

echo
export cancellation_time=2024-12-01T00:00:00Z
resim set-current-time $cancellation_time
resim call-method ${component} reservation_cancellation_by_customer ${reservation}:${reservation_id} >$OUTPUTFILE || ( cat $OUTPUTFILE ; exit 1 )
export refund_amount=$(grep -A 1 "ResAddr: $accepted_coin_address" $OUTPUTFILE | head -n 8 | tail -n 1 | cut -d ':' -f 2)
echo "Reservation ${reservation_id} cancelled by the customer on ${cancellation_time}, ${refund_amount} XRD received (should be ${payment_amount})"
