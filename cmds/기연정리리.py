# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [기연이름] 기연정리')
            return
            
        msg = ''
        if line not in ONEITEM.index:
            ob.sendLine('☞ 그런 아이템은 없습니다.!')
            return
        index = ONEITEM.index[line]
        owner = ONEITEM[index]
        words = owner.split()
        if len(words) == 1:
            who = owner
            where = ''
        elif len(words) == 3:
            who = words[0]
            where = words[1]
        else:
            ob.sendLine('아무도 소지하고 있지 않습니다.!')
            return
        for obj in ob.channel.players:
            if obj['이름'] == who:
                ob.sendLine('사용자가 접속중입니다.!')
                return
        if where == '':
            player = Player()
            if player.load(who) == False:
                ob.sendLine('존재하지않는 사용자입니다.')
                return
            last = player['마지막저장시간']
            if last != '' and time.time() - last < 259200:
                ob.sendLine('아직 3일이 경과하지 않았습니다.')
                return
            for obj in player.objs:
                print(obj['이름'])
                if obj.index == index:
                    player.objs.remove(obj)
                    player.save(False)
                    ob.sendLine('%s의 %s%s 정리하였습니다.' % (who, line, han_obj(line)))
                    del player
                    ONEITEM.attr.__delitem__(index)
                    ONEITEM.save()
                    return
        else:
            pass
        ob.sendLine('%s %s' % (who, where))
        ONEITEM.attr.__delitem__(index)
        ONEITEM.save()

