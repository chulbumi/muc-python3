# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [대상] 동행')
            return
        if line == '나' or line == ob['이름']:
            if ob.Party != None:
                leader = ob.Party
                if leader  == ob:
                    ob.sendLine('무리의 대장은 이탈할수 없어요.')
                    return
                else:
                    ob.follow = None
                    ob.Party = None
                    leader.follower.remove(ob)
                    leader.PartyMember.remove(ob)
                    leader.sendToParty('당신의 무리에서 [1m%s[0m[40m[37m%s 나갔습니다.' % (ob['이름'], han_iga(ob['이름'])), ex = leader, prompt = True)
                    leader.sendLine('\r\n당신의 무리에서 [1m%s[0m[40m[37m%s 나갔습니다.' % (ob['이름'], han_iga(ob['이름'])))
                    ob.sendLine('당신은 [1m%s[0m[40m[37m의 무리에서 이탈 하였습니다.' % leader['이름'])
                    if len(leader.PartyMember) == 0:
                        leader.PartyMember=[]
                        leader.Party = None
                        leader.sendLine('\r\n무리가 해제 되었습니다')
                    leader.lpPrompt()
            else:
                ob.delFollow()
                ob.sendLine('당신은 홀로 강호를 주유하기 시작합니다.')
            return
        target = ob.env.findObjName(line)
        if target == None or is_player(target) == False:
            ob.sendLine('☞ 그런 대상이 없어요. ^^')
            return
        if target.checkConfig('동행거부'):
            ob.sendLine('%s 동행거부중 입니다.' % target.han_iga())
            return
        ob.delFollow()
        ob.addFollow(target)

