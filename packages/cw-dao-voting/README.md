# Voting

This package collects functionality that is common across proposal
modules and voting. A description of each module in this package
follows.

## Deposit

This module provides types and methods for collecting and returning
proposal deposits.

## Proposal

This module provides a simple trait that proposal modules may
implement for their proposal struct to make it compatible with other
methods in this package.

## Reply

This provides methods for tagging reply IDs. Doing so allows us to
encode information about the type of reply we're getting in the ID,
thus allowing us to handle multiple types of replies.

## Status

This module provides a `Status` enum which proposal module will likely
want to use to describe proposal states. It also provides a `Display`
implementation for this status which allows conversion of status to a
string. This conversion is helpful for the `proposal-hooks` package
which uses a string for the proposal status to be sufficently generic.

## Threshold

This module provides threshold types and corresponding
validation. Proposal module authors will likely want to use this 
package when defining passing thresholds for their proposals.

## Voting

This module provides methods for determining proposal outcomes. We've
spent a very considerable amount of time making sure that these
methods are solid. These methods can be used in conjunction with the
threshold types provided by this package.
