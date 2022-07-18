# CW721 Controllers: Common cw721 controllers for many contracts

This is a collection of cw721 "controllers" that we end up reimplementing in 
many contracts. I use the word "controller" similar to the MVC framework
style, where it is an element that encapsulated business logic and data access.
We can also directly handle some `ExecuteMsg` and `QueryMsg` variants by
adding a sub-router to these controllers.

This is the beginning of an experiment in code composition, and how best to
reuse code among multiple contracts.
