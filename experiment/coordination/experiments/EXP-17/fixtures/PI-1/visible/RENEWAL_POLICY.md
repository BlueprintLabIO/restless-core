# Renewal action policy v3

Apply the first matching safety rule, then the commercial rules.

1. `security_complaint=true`: state `hold`; next action must say to open a security escalation before
   commercial outreach; owner `security-response`.
2. `payment_overdue=true`: state `hold`; next action must request a finance/account-status check and
   must not promise renewal terms; owner `finance-operations`.
3. Renewal in 30 days or fewer with either usage decline of at least 30% or an unresolved critical
   support case: state `risk`; next action must name an evidence review and recovery meeting; owner
   `account-owner`.
4. Usage growth of at least 25%, no unresolved high/critical case, and renewal more than 30 days away:
   state `opportunity`; next action must validate expansion needs; owner `account-owner`.
5. Otherwise state `stable`; next action must schedule the normal renewal check-in; owner
   `account-owner`.

Confidence is high when usage, support, correspondence and renewal timing are all present and agree;
medium when one is absent or mixed; low when two or more are absent. Evidence may cite only exact
`source_id` values inside the same account. Facts not in the records belong in `unknowns`.
