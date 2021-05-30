/* 
apiVersion: v1
kind: Endpoints
metadata:
  creationTimestamp: 2019-03-20T07:23:28Z
  name: account
  namespace: default
  resourceVersion: "82479279"
  selfLink: /api/v1/namespaces/default/endpoints/account
  uid: 0ec10531-4ae1-11e9-9c9c-f86eee307061
subsets:
- addresses:
  - ip: 172.16.61.84
  - ip: 172.16.61.85
  - ip: 172.16.61.86
  - ip: 172.16.61.87
  - ip: 172.16.61.88
  - ip: 172.16.61.90
  ports:
  - name: port80
    port: 31000
    protocol: TCP
  - name: port82
    port: 31002
    protocol: TCP
  - name: port81
    port: 31001
    protocol: TCP
*/
