# ENV VARIABLES
### RPC_URL - HTTP URL TO ETHERIUM NODE
### DATABASE_URL - URL TO POSTGRES DB
### MAX_ALLOWED_GAS - MAXIMUM TOTAL GAS PRICE IN WEI
### MAX_PRIORITY_FEE - PRIORITY FEE PRICE IN WEI

# API.
## Invoices States:
  0 => Empty,
  1 => Incomplete,
  2 => Complete,
  3 => Rejected,
  4 => Sent,
## Invoice Actions:
  0 => SendToReceiver,
  1 => Nothing,

## GET get_by_status/{status: number} => Returns list of invoiced with provided status
## GET get_by_action/{action: number} => Returns list of invoices with provided action
## GET get_by_address/{address: string} => Returns invoice by wallet address
## GET manual_check/{address: string} => Refresh and returns invoice state by wallet address
## POST create_invoice body:
```json
{
    "receiver": "0x68fe0e9b614894b1A537bf6FB054331BAc63092a", //reciver wallet
    "value": 0.0037, // value in eth
    "lifetime": 900, // lifetime in seconds
    "action": 0 // OPTIONAL! Invoice action present in number
}
```
Returns invoice wallet address
